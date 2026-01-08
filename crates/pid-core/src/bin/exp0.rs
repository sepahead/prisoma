use pid_core::{
    co_information_pairwise, concat_horiz, distance_concentration_stats,
    intrinsic_dimension_levina_bickel, isx_redundancy, ksg_mi, ksg_mi_concat_xy,
    DistanceConcentrationConfig, HashProjector, IntrinsicDimConfig, IsxConfig, IsxMethod,
    KsgConfig, MatRef, Metric, NegativeHandling, PcaProjector, Standardizer,
};
use std::io::{self, Write};

#[derive(Debug, Clone, Copy)]
struct Args {
    csv: bool,
}

#[derive(Debug, Clone, Copy)]
struct CaseCommon<'a> {
    args: Args,
    n: usize,
    ksg_cfg: &'a KsgConfig,
    hash_project_to: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct CaseSpec<'a> {
    name: &'a str,
    d: usize,
    seed: u64,
}

#[derive(Debug)]
enum Exp0Error {
    Pid(pid_core::PidError),
    Io(io::Error),
}

impl From<pid_core::PidError> for Exp0Error {
    fn from(value: pid_core::PidError) -> Self {
        Self::Pid(value)
    }
}

impl From<io::Error> for Exp0Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn main() {
    let args = match parse_args() {
        Ok(Some(a)) => a,
        Ok(None) => {
            let mut out = io::BufWriter::new(io::stdout());
            if let Err(e) = print_usage(&mut out) {
                // If someone does `exp0 --help | head`, avoid panicking.
                if e.kind() == io::ErrorKind::BrokenPipe {
                    return;
                }
                eprintln!("exp0: failed to write help: {e}");
            }
            return;
        }
        Err(msg) => {
            eprintln!("exp0: {msg}");
            eprintln!();
            let mut out = io::BufWriter::new(io::stderr());
            let _ = print_usage(&mut out);
            std::process::exit(2);
        }
    };

    let mut out = io::BufWriter::new(io::stdout());
    if let Err(err) = run(&mut out, args) {
        match err {
            Exp0Error::Io(e) if e.kind() == io::ErrorKind::BrokenPipe => (),
            Exp0Error::Pid(e) => {
                eprintln!("exp0: estimator error: {e}");
                std::process::exit(1);
            }
            Exp0Error::Io(e) => {
                eprintln!("exp0: IO error: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut csv = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--csv" => csv = true,
            "--help" | "-h" => return Ok(None),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Some(Args { csv }))
}

fn print_usage(out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "Usage: exp0 [--csv]")?;
    writeln!(out)?;
    writeln!(out, "  --csv   Emit machine-readable CSV (two tables).")?;
    writeln!(out, "  -h, --help   Show this help.")?;
    Ok(())
}

fn run(out: &mut dyn Write, args: Args) -> Result<(), Exp0Error> {
    // Minimal Experiment 0 runner (Rust-side).
    //
    // This is intentionally small and brute-force; it exists to exercise the estimators end-to-end
    // on synthetic systems and to provide a place to iterate while building the full harness.

    let n = 500usize;
    let k = 3usize;
    let dims = [10usize, 64, 256];
    let hash_project_to = Some(64usize);

    let ksg_cfg = KsgConfig {
        k,
        metric: Metric::Chebyshev,
        tie_epsilon: 0.0,
        negative_handling: NegativeHandling::ClampToZero,
    };

    if args.csv {
        write_case_csv_header(out)?;
    } else {
        writeln!(out, "Experiment 0 (Rust quick run)")?;
        writeln!(out, "n={n}, k={k}, dims={dims:?}")?;
        writeln!(
            out,
            "project_to={hash_project_to:?} (projection baselines: hash + PCA; S1,S2 only)"
        )?;
        writeln!(out)?;
    }

    let mut passes = 0;
    let mut total = 0;
    let mut geom_warnings = 0;

    let common = CaseCommon {
        args,
        n,
        ksg_cfg: &ksg_cfg,
        hash_project_to,
    };
    for d in dims {
        for name in ["independent_additive", "redundant_copy", "unique_s1", "xor_like"] {
            let res = run_case(
                out,
                common,
                CaseSpec {
                    name,
                    d,
                    seed: 42,
                },
            )?;

            // Simple heuristic check for Independent Additive case (True MI is small but non-zero).
            // For Redundant Copy, Red should be large.
            // For Exp0 validation, we want to know if it's "plausible".
            if name == "independent_additive" {
                total += 1;
                let rel_err = (res.metrics.red_ehrlich).abs(); // Ideally 0
                let threshold = if d <= 10 { 0.1 } else if d <= 100 { 0.2 } else { 0.3 };
                if rel_err < threshold {
                    passes += 1;
                }

                // Check geometry warnings on the independent additive case as a proxy for the dimension
                let dr_s1 = 2.0 * res.diag.gromov_s1 / res.diag.diam_s1;
                if res.diag.id_s1 > 20.0 || res.diag.dc_cv_s1 < 0.1 || dr_s1 < 0.1 {
                    geom_warnings += 1;
                }
            }
        }
        if !common.args.csv {
            writeln!(out)?;
        }
    }

    if common.args.csv {
        writeln!(out)?;
        write_gaussian_csv_header(out)?;
    }
    run_gaussian_channel_strong_dependence_sweep(out, common.args, 900, &ksg_cfg, 0x51A7_2026)?;

    if !args.csv {
        writeln!(out, "--- Experiment 0 Summary ---")?;
        writeln!(out, "Passes (Independent Additive Zero-Redundancy check): {}/{}", passes, total)?;
        writeln!(out, "Geometry Warnings: {}/{}", geom_warnings, dims.len())?;
        let status = if passes == total && geom_warnings == 0 {
            "GO"
        } else if passes as f64 / total as f64 > 0.5 {
            "PIVOT (High-d instability or geometry risks detected)"
        } else {
            "NO-GO"
        };
        writeln!(out, "Status: {}", status)?;
    }

    Ok(())
}

fn run_case(
    out: &mut dyn Write,
    common: CaseCommon<'_>,
    spec: CaseSpec<'_>,
) -> Result<CaseResult, Exp0Error> {
    let noise_std = 0.05;
    let n = common.n;
    let d = spec.d;
    let seed = spec.seed;
    let (s1, s2, t) = match spec.name {
        "independent_additive" => gen_independent_additive(n, d, noise_std, seed),
        "redundant_copy" => gen_redundant_copy(n, d, noise_std, seed),
        "unique_s1" => gen_unique_s1(n, d, noise_std, seed),
        "xor_like" => gen_xor_like(n, d, noise_std, seed),
        _ => unreachable!("unknown case: {}", spec.name),
    };

    let s1 = MatRef::new(&s1, n, d)?;
    let s2 = MatRef::new(&s2, n, d)?;
    let t = MatRef::new(&t, n, 1)?;

    let (s1z, _) = Standardizer::fit_transform(s1)?;
    let (s2z, _) = Standardizer::fit_transform(s2)?;
    let (tz, _) = Standardizer::fit_transform(t)?;

    let baseline = compute_metrics(s1z.as_ref(), s2z.as_ref(), tz.as_ref(), common.ksg_cfg)?;
    let diag = compute_diagnostics(
        s1z.as_ref(),
        s2z.as_ref(),
        tz.as_ref(),
        common.ksg_cfg.metric,
    );

    if common.args.csv {
        write_case_csv_row(
            out,
            common.ksg_cfg,
            CaseCsvRow {
                name: spec.name,
                projection: ProjectionMethod::None,
                d,
                n,
                project_to: None,
                metrics: baseline,
                diag,
            },
        )?;
    } else {
        print_metrics(out, spec.name, d, baseline)?;
        print_intrinsic_dims(out, diag)?;
    }

    if let Some(dout) = common.hash_project_to {
        if d > dout {
            let p1 = HashProjector::new(d, dout, 0xA11CE_u64 ^ seed)?;
            let p2 = HashProjector::new(d, dout, 0xB22CE_u64 ^ seed)?;

            let s1p = p1.transform(s1z.as_ref())?;
            let s2p = p2.transform(s2z.as_ref())?;

            // Re-standardize after projection so Chebyshev distance has comparable scale.
            let (s1p, _) = Standardizer::fit_transform(s1p.as_ref())?;
            let (s2p, _) = Standardizer::fit_transform(s2p.as_ref())?;

            let projected =
                compute_metrics(s1p.as_ref(), s2p.as_ref(), tz.as_ref(), common.ksg_cfg)?;
            let diag_p = compute_diagnostics(
                s1p.as_ref(),
                s2p.as_ref(),
                tz.as_ref(),
                common.ksg_cfg.metric,
            );
            let case_name = format!("{}_hashproj", spec.name);
            if common.args.csv {
                write_case_csv_row(
                    out,
                    common.ksg_cfg,
                    CaseCsvRow {
                        name: &case_name,
                        projection: ProjectionMethod::Hash,
                        d: dout,
                        n,
                        project_to: Some(dout),
                        metrics: projected,
                        diag: diag_p,
                    },
                )?;
            } else {
                print_metrics(out, &case_name, dout, projected)?;
                print_intrinsic_dims(out, diag_p)?;
            }

            // PCA projection baseline (deterministic; no external deps).
            let (s1p, _) = PcaProjector::fit_transform(s1z.as_ref(), dout)?;
            let (s2p, _) = PcaProjector::fit_transform(s2z.as_ref(), dout)?;

            // Re-standardize after projection so Chebyshev distance has comparable scale.
            let (s1p, _) = Standardizer::fit_transform(s1p.as_ref())?;
            let (s2p, _) = Standardizer::fit_transform(s2p.as_ref())?;

            let projected =
                compute_metrics(s1p.as_ref(), s2p.as_ref(), tz.as_ref(), common.ksg_cfg)?;
            let diag_p = compute_diagnostics(
                s1p.as_ref(),
                s2p.as_ref(),
                tz.as_ref(),
                common.ksg_cfg.metric,
            );
            let case_name = format!("{}_pca", spec.name);
            if common.args.csv {
                write_case_csv_row(
                    out,
                    common.ksg_cfg,
                    CaseCsvRow {
                        name: &case_name,
                        projection: ProjectionMethod::Pca,
                        d: dout,
                        n,
                        project_to: Some(dout),
                        metrics: projected,
                        diag: diag_p,
                    },
                )?;
            } else {
                print_metrics(out, &case_name, dout, projected)?;
                print_intrinsic_dims(out, diag_p)?;
            }
        }
    }
    Ok(CaseResult {
        metrics: baseline,
        diag,
    })
}

struct CaseResult {
    metrics: Metrics,
    diag: Diagnostics,
}

#[derive(Debug, Clone, Copy)]
struct Diagnostics {
    id_s1: f64,
    id_s2: f64,
    id_t: f64,
    id_s12: f64,

    dc_cv_s1: f64,
    dc_nnr_s1: f64,
    dc_cv_s2: f64,
    dc_nnr_s2: f64,
    dc_cv_s12: f64,
    dc_nnr_s12: f64,

    gromov_s1: f64,
    gromov_s2: f64,
    gromov_s12: f64,
    gromov_t: f64,

    diam_s1: f64,
    diam_s2: f64,
    diam_s12: f64,
    diam_t: f64,
}

fn compute_diagnostics(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    metric: Metric,
) -> Diagnostics {
    let cfg = IntrinsicDimConfig { k: 10, metric };

    let id_s1 = intrinsic_dimension_levina_bickel(s1, &cfg).unwrap_or(f64::NAN);
    let id_s2 = intrinsic_dimension_levina_bickel(s2, &cfg).unwrap_or(f64::NAN);
    let id_t = intrinsic_dimension_levina_bickel(t, &cfg).unwrap_or(f64::NAN);
    let id_s12 = concat_horiz(s1, s2)
        .ok()
        .and_then(|s12| intrinsic_dimension_levina_bickel(s12.as_ref(), &cfg).ok())
        .unwrap_or(f64::NAN);

    let dcfg = DistanceConcentrationConfig { metric };
    let ds1 = distance_concentration_stats(s1, &dcfg).ok();
    let ds2 = distance_concentration_stats(s2, &dcfg).ok();
    let ds12 = concat_horiz(s1, s2)
        .ok()
        .and_then(|s12| distance_concentration_stats(s12.as_ref(), &dcfg).ok());
    let dt = distance_concentration_stats(t, &dcfg).ok();

    let hcfg = pid_core::HyperbolicityConfig {
        n_samples: 500,
        metric,
        seed: 42,
    };
    let gromov_s1 = pid_core::gromov_hyperbolicity(s1, &hcfg).unwrap_or(f64::NAN);
    let gromov_s2 = pid_core::gromov_hyperbolicity(s2, &hcfg).unwrap_or(f64::NAN);
    let gromov_t = pid_core::gromov_hyperbolicity(t, &hcfg).unwrap_or(f64::NAN);
    let gromov_s12 = concat_horiz(s1, s2)
        .ok()
        .and_then(|s12| pid_core::gromov_hyperbolicity(s12.as_ref(), &hcfg).ok())
        .unwrap_or(f64::NAN);

    Diagnostics {
        id_s1,
        id_s2,
        id_t,
        id_s12,
        dc_cv_s1: ds1.map(|s| s.pairwise_cv).unwrap_or(f64::NAN),
        dc_nnr_s1: ds1.map(|s| s.nn_over_pairwise_mean).unwrap_or(f64::NAN),
        dc_cv_s2: ds2.map(|s| s.pairwise_cv).unwrap_or(f64::NAN),
        dc_nnr_s2: ds2.map(|s| s.nn_over_pairwise_mean).unwrap_or(f64::NAN),
        dc_cv_s12: ds12.map(|s| s.pairwise_cv).unwrap_or(f64::NAN),
        dc_nnr_s12: ds12.map(|s| s.nn_over_pairwise_mean).unwrap_or(f64::NAN),
        gromov_s1,
        gromov_s2,
        gromov_s12,
        gromov_t,
        diam_s1: ds1.map(|s| s.pairwise_max).unwrap_or(f64::NAN),
        diam_s2: ds2.map(|s| s.pairwise_max).unwrap_or(f64::NAN),
        diam_s12: ds12.map(|s| s.pairwise_max).unwrap_or(f64::NAN),
        diam_t: dt.map(|s| s.pairwise_max).unwrap_or(f64::NAN),
    }
}

fn print_intrinsic_dims(out: &mut dyn Write, d: Diagnostics) -> io::Result<()> {
    writeln!(
        out,
        "{:>20} {:>7} | ID(s1)={:>6.2} ID(s2)={:>6.2} ID(t)={:>6.2} ID(s1,s2)={:>6.2}",
        "", "", d.id_s1, d.id_s2, d.id_t, d.id_s12
    )?;
    writeln!(
        out,
        "{:>20} {:>7} | DCcv(s1)={:>6.3} nn/mean={:>6.3} | DCcv(s2)={:>6.3} nn/mean={:>6.3} | DCcv(s1,s2)={:>6.3} nn/mean={:>6.3}",
        "",
        "",
        d.dc_cv_s1,
        d.dc_nnr_s1,
        d.dc_cv_s2,
        d.dc_nnr_s2,
        d.dc_cv_s12,
        d.dc_nnr_s12
    )?;

    let dr_s1 = 2.0 * d.gromov_s1 / d.diam_s1;
    let dr_s2 = 2.0 * d.gromov_s2 / d.diam_s2;
    let dr_s12 = 2.0 * d.gromov_s12 / d.diam_s12;
    let dr_t = 2.0 * d.gromov_t / d.diam_t;

    writeln!(
        out,
        "{:>20} {:>7} | d_rel(s1)={:>6.3} | d_rel(s2)={:>6.3} | d_rel(s1,s2)={:>6.3} | d_rel(t)={:>6.3}",
        "", "", dr_s1, dr_s2, dr_s12, dr_t
    )?;
    Ok(())
}

fn run_gaussian_channel_strong_dependence_sweep(
    out: &mut dyn Write,
    args: Args,
    n: usize,
    ksg_cfg: &KsgConfig,
    seed: u64,
) -> Result<(), Exp0Error> {
    // Strong-dependence sweep (separate axis from "high d"):
    // X ~ N(0,1), Y = X + σN, N~N(0,1), so analytic MI is:
    // I(X;Y) = 0.5 ln(1 + 1/σ²).
    let sigmas = [1.0, 0.3, 0.1, 0.03, 0.01];

    let mut rng = Rng64::new(seed);
    let mut x = Vec::with_capacity(n);
    let mut noise = Vec::with_capacity(n);
    for _ in 0..n {
        x.push(rng.normal());
        noise.push(rng.normal());
    }

    let xref = MatRef::new(&x, n, 1)?;
    let (xstd, _) = Standardizer::fit_transform(xref)?;

    if !args.csv {
        writeln!(out, "Strong-dependence sweep (Gaussian channel, 1D)")?;
        writeln!(out, "n={n}, k={}, metric={:?}", ksg_cfg.k, ksg_cfg.metric)?;
    }
    for &sigma in &sigmas {
        let mut y = Vec::with_capacity(n);
        for (&xi, &ni) in x.iter().zip(noise.iter()) {
            y.push(xi + sigma * ni);
        }

        let yref = MatRef::new(&y, n, 1)?;
        let (ystd, _) = Standardizer::fit_transform(yref)?;

        let mi_hat = ksg_mi(xstd.as_ref(), ystd.as_ref(), ksg_cfg)?;
        let mi_true = gaussian_channel_mi(sigma);
        if args.csv {
            write_gaussian_csv_row(out, sigma, n, ksg_cfg, mi_hat, mi_true)?;
        } else {
            writeln!(
                out,
                "  sigma={:<7.3}  MI_hat={:>8.3}  MI_true={:>8.3}  err={:>8.3}",
                sigma,
                mi_hat,
                mi_true,
                mi_hat - mi_true
            )?;
        }
    }
    if !args.csv {
        writeln!(out)?;
    }
    Ok(())
}

fn gaussian_channel_mi(sigma: f64) -> f64 {
    debug_assert!(sigma.is_finite());
    debug_assert!(sigma > 0.0);
    0.5 * (1.0 + 1.0 / (sigma * sigma)).ln()
}

#[derive(Clone, Copy)]
struct Metrics {
    mi_s1_t: f64,
    mi_s2_t: f64,
    mi_s1s2_t: f64,
    ci: f64,
    red_ehrlich: f64,
    red_local_min: f64,
    red_disjunction: f64,
    syn_ehrlich: f64,
}

fn compute_metrics(
    s1: MatRef<'_>,
    s2: MatRef<'_>,
    t: MatRef<'_>,
    ksg_cfg: &KsgConfig,
) -> pid_core::PidResult<Metrics> {
    let mi_s1_t = ksg_mi(s1, t, ksg_cfg)?;
    let mi_s2_t = ksg_mi(s2, t, ksg_cfg)?;
    let mi_s1s2_t = ksg_mi_concat_xy(s1, s2, t, ksg_cfg)?;
    let ci = co_information_pairwise(s1, s2, t, ksg_cfg)?;

    let red_ehrlich = isx_redundancy(
        s1,
        s2,
        t,
        &IsxConfig {
            k: ksg_cfg.k,
            metric: ksg_cfg.metric,
            tie_epsilon: ksg_cfg.tie_epsilon,
            method: IsxMethod::EhrlichKsg,
        },
    )?;

    let red_local_min = isx_redundancy(
        s1,
        s2,
        t,
        &IsxConfig {
            k: ksg_cfg.k,
            metric: ksg_cfg.metric,
            tie_epsilon: ksg_cfg.tie_epsilon,
            method: IsxMethod::LocalMinKsg,
        },
    )?;

    let red_disjunction = isx_redundancy(
        s1,
        s2,
        t,
        &IsxConfig {
            k: ksg_cfg.k,
            metric: ksg_cfg.metric,
            tie_epsilon: ksg_cfg.tie_epsilon,
            method: IsxMethod::DisjunctionFromLocalMi,
        },
    )
    .unwrap_or(f64::NAN);

    Ok(Metrics {
        mi_s1_t,
        mi_s2_t,
        mi_s1s2_t,
        ci,
        red_ehrlich,
        red_local_min,
        red_disjunction,
        syn_ehrlich: mi_s1s2_t - mi_s1_t - mi_s2_t + red_ehrlich,
    })
}

fn print_metrics(out: &mut dyn Write, name: &str, d: usize, m: Metrics) -> io::Result<()> {
    writeln!(
        out,
        "{name:>20} d={d:<4} | I1={:>7.3} I2={:>7.3} I12={:>7.3} CI={:>7.3} | Red(ehrlich)={:>7.3} Red(local_min)={:>7.3} Red(disj)={:>7.3} | Syn(ehrlich)={:>7.3}",
        m.mi_s1_t,
        m.mi_s2_t,
        m.mi_s1s2_t,
        m.ci,
        m.red_ehrlich,
        m.red_local_min,
        m.red_disjunction,
        m.syn_ehrlich,
    )?;
    Ok(())
}

fn write_case_csv_header(out: &mut dyn Write) -> io::Result<()> {
    writeln!(
        out,
        "case_name,projection,d,n,k,metric,project_to,mi_s1_t,mi_s2_t,mi_s1s2_t,ci,red_ehrlich,red_local_min,red_disjunction,syn_ehrlich,id_s1,id_s2,id_t,id_s12,dc_cv_s1,dc_nnratio_s1,dc_cv_s2,dc_nnratio_s2,dc_cv_s12,dc_nnratio_s12,gromov_s1,gromov_s2,gromov_s12,gromov_t,dr_s1,dr_s2,dr_s12,dr_t"
    )
}

#[derive(Clone, Copy)]
enum ProjectionMethod {
    None,
    Hash,
    Pca,
}

impl ProjectionMethod {
    fn as_str(self) -> &'static str {
        match self {
            ProjectionMethod::None => "none",
            ProjectionMethod::Hash => "hash",
            ProjectionMethod::Pca => "pca",
        }
    }
}

struct CaseCsvRow<'a> {
    name: &'a str,
    projection: ProjectionMethod,
    d: usize,
    n: usize,
    project_to: Option<usize>,
    metrics: Metrics,
    diag: Diagnostics,
}

fn write_case_csv_row(
    out: &mut dyn Write,
    ksg_cfg: &KsgConfig,
    row: CaseCsvRow<'_>,
) -> io::Result<()> {
    let project_to = row.project_to.map_or_else(String::new, |v| v.to_string());
    let dr_s1 = 2.0 * row.diag.gromov_s1 / row.diag.diam_s1;
    let dr_s2 = 2.0 * row.diag.gromov_s2 / row.diag.diam_s2;
    let dr_s12 = 2.0 * row.diag.gromov_s12 / row.diag.diam_s12;
    let dr_t = 2.0 * row.diag.gromov_t / row.diag.diam_t;

    writeln!(
        out,
        "{},{},{},{},{},{:?},{project_to},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e}",
        row.name,
        row.projection.as_str(),
        row.d,
        row.n,
        ksg_cfg.k,
        ksg_cfg.metric,
        row.metrics.mi_s1_t,
        row.metrics.mi_s2_t,
        row.metrics.mi_s1s2_t,
        row.metrics.ci,
        row.metrics.red_ehrlich,
        row.metrics.red_local_min,
        row.metrics.red_disjunction,
        row.metrics.syn_ehrlich,
        row.diag.id_s1,
        row.diag.id_s2,
        row.diag.id_t,
        row.diag.id_s12,
        row.diag.dc_cv_s1,
        row.diag.dc_nnr_s1,
        row.diag.dc_cv_s2,
        row.diag.dc_nnr_s2,
        row.diag.dc_cv_s12,
        row.diag.dc_nnr_s12,
        row.diag.gromov_s1,
        row.diag.gromov_s2,
        row.diag.gromov_s12,
        row.diag.gromov_t,
        dr_s1,
        dr_s2,
        dr_s12,
        dr_t,
    )
}

fn write_gaussian_csv_header(out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "sigma,n,k,metric,mi_hat,mi_true,err")
}

fn write_gaussian_csv_row(
    out: &mut dyn Write,
    sigma: f64,
    n: usize,
    ksg_cfg: &KsgConfig,
    mi_hat: f64,
    mi_true: f64,
) -> io::Result<()> {
    writeln!(
        out,
        "{sigma:.15e},{n},{},{:?},{mi_hat:.15e},{mi_true:.15e},{:.15e}",
        ksg_cfg.k,
        ksg_cfg.metric,
        mi_hat - mi_true
    )
}

fn gen_independent_additive(
    n: usize,
    d: usize,
    noise_std: f64,
    seed: u64,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut rng = Rng64::new(seed);
    let mut s1 = vec![0.0; n * d];
    let mut s2 = vec![0.0; n * d];
    let mut t = vec![0.0; n];

    for i in 0..n {
        for j in 0..d {
            s1[i * d + j] = rng.normal();
            s2[i * d + j] = rng.normal();
        }
        t[i] = s1[i * d] + s2[i * d] + noise_std * rng.normal();
    }
    (s1, s2, t)
}

fn gen_redundant_copy(
    n: usize,
    d: usize,
    noise_std: f64,
    seed: u64,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut rng = Rng64::new(seed);
    let mut s1 = vec![0.0; n * d];
    let mut s2 = vec![0.0; n * d];
    let mut t = vec![0.0; n];

    for i in 0..n {
        let base = rng.normal();
        t[i] = base;
        s1[i * d] = base + noise_std * rng.normal();
        s2[i * d] = base + noise_std * rng.normal();
        for j in 1..d {
            s1[i * d + j] = rng.normal();
            s2[i * d + j] = rng.normal();
        }
    }
    (s1, s2, t)
}

fn gen_unique_s1(n: usize, d: usize, noise_std: f64, seed: u64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut rng = Rng64::new(seed);
    let mut s1 = vec![0.0; n * d];
    let mut s2 = vec![0.0; n * d];
    let mut t = vec![0.0; n];

    for i in 0..n {
        for j in 0..d {
            s1[i * d + j] = rng.normal();
            s2[i * d + j] = rng.normal();
        }
        t[i] = s1[i * d] + noise_std * rng.normal();
    }
    (s1, s2, t)
}

fn gen_xor_like(n: usize, d: usize, noise_std: f64, seed: u64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut rng = Rng64::new(seed);
    let mut s1 = vec![0.0; n * d];
    let mut s2 = vec![0.0; n * d];
    let mut t = vec![0.0; n];

    for i in 0..n {
        let a = rng.normal();
        let b = rng.normal();
        s1[i * d] = a;
        s2[i * d] = b;

        // XOR-like: target depends on the interaction sign(a*b) rather than either alone.
        let sign = if a * b > 0.0 { 1.0 } else { -1.0 };
        t[i] = sign + noise_std * rng.normal();

        for j in 1..d {
            s1[i * d + j] = rng.normal();
            s2[i * d + j] = rng.normal();
        }
    }
    (s1, s2, t)
}

#[derive(Clone)]
struct Rng64 {
    state: u64,
}

impl Rng64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn next_f64(&mut self) -> f64 {
        let u = self.next_u64() >> 11; // 53 bits
        (u as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    fn normal(&mut self) -> f64 {
        // Box–Muller.
        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        r * theta.cos()
    }
}