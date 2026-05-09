use pid_core::{
    average_degree_of_redundancy, average_degree_of_vulnerability, co_information_pairwise,
    concat_horiz, distance_concentration_stats, intrinsic_dimension_levina_bickel, isx_redundancy,
    ksg_mi, ksg_mi_concat_xy, DistanceConcentrationConfig, HashProjector, IntrinsicDimConfig,
    IsxConfig, IsxMethod, KsgConfig, MatRef, Metric, NegativeHandling, PcaProjector, Standardizer,
};
use std::fs::File;
use std::io::{self, Write};

#[derive(Debug, Clone)]
struct Args {
    csv: bool,
    seeds: usize,
    summary_json: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct CaseCommon<'a> {
    csv: bool,
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
    let mut seeds = 3usize;
    let mut summary_json = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--csv" => csv = true,
            "--seeds" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--seeds requires a positive integer".to_string())?;
                seeds = raw
                    .parse::<usize>()
                    .map_err(|_| "--seeds requires a positive integer".to_string())?;
                if seeds == 0 {
                    return Err("--seeds requires a positive integer".to_string());
                }
            }
            "--summary-json" => {
                summary_json = Some(
                    args.next()
                        .ok_or_else(|| "--summary-json requires a path".to_string())?,
                );
            }
            "--help" | "-h" => return Ok(None),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Some(Args {
        csv,
        seeds,
        summary_json,
    }))
}

fn print_usage(out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "Usage: exp0 [--csv] [--seeds N] [--summary-json PATH]")?;
    writeln!(out)?;
    writeln!(out, "  --csv   Emit machine-readable CSV (two tables).")?;
    writeln!(
        out,
        "  --seeds N   Run N deterministic seeds per case (default: 3)."
    )?;
    writeln!(
        out,
        "  --summary-json PATH   Write gate summary metadata as JSON."
    )?;
    writeln!(out, "  -h, --help   Show this help.")?;
    Ok(())
}

fn make_seeds(n: usize) -> Vec<u64> {
    (0..n)
        .map(|i| 42u64.wrapping_add((i as u64).wrapping_mul(1_000_003)))
        .collect()
}

fn write_summary_json(
    path: &str,
    gates: &GateSummary,
    n: usize,
    k: usize,
    dims: &[usize],
    seeds: &[u64],
    hash_project_to: Option<usize>,
) -> io::Result<()> {
    let mut file = File::create(path)?;
    let config_hash = config_hash(n, k, dims, seeds, hash_project_to);
    writeln!(file, "{{")?;
    writeln!(file, "  \"config_hash\": \"{config_hash:016x}\",")?;
    writeln!(file, "  \"n\": {n},")?;
    writeln!(file, "  \"k\": {k},")?;
    writeln!(file, "  \"dims\": {},", json_usize_array(dims))?;
    writeln!(file, "  \"seeds\": {},", json_u64_array(seeds))?;
    match hash_project_to {
        Some(v) => writeln!(file, "  \"hash_project_to\": {v},")?,
        None => writeln!(file, "  \"hash_project_to\": null,")?,
    }
    writeln!(file, "  \"case_results\": {},", gates.case_results)?;
    writeln!(file, "  \"red_zero_checks\": {},", gates.red_zero_checks)?;
    writeln!(file, "  \"red_zero_passes\": {},", gates.red_zero_passes)?;
    writeln!(
        file,
        "  \"monotonicity_violations\": {},",
        gates.monotonicity_violations
    )?;
    writeln!(file, "  \"cmi_violations\": {},", gates.cmi_violations)?;
    writeln!(
        file,
        "  \"invariant_violations\": {},",
        gates.invariant_violations
    )?;
    writeln!(
        file,
        "  \"geometry_warnings\": {},",
        gates.geometry_warnings
    )?;
    writeln!(file, "  \"status\": \"{}\"", gates.status())?;
    writeln!(file, "}}")?;
    Ok(())
}

fn json_usize_array(values: &[usize]) -> String {
    let parts: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn json_u64_array(values: &[u64]) -> String {
    let parts: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn config_hash(
    n: usize,
    k: usize,
    dims: &[usize],
    seeds: &[u64],
    hash_project_to: Option<usize>,
) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    mix_u64(&mut h, n as u64);
    mix_u64(&mut h, k as u64);
    mix_u64(&mut h, dims.len() as u64);
    for &d in dims {
        mix_u64(&mut h, d as u64);
    }
    mix_u64(&mut h, seeds.len() as u64);
    for &seed in seeds {
        mix_u64(&mut h, seed);
    }
    mix_u64(&mut h, hash_project_to.map_or(u64::MAX, |v| v as u64));
    h
}

fn mix_u64(h: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        *h ^= byte as u64;
        *h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
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
    let seeds = make_seeds(args.seeds);

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
        writeln!(out, "n={n}, k={k}, dims={dims:?}, seeds={seeds:?}")?;
        writeln!(
            out,
            "project_to={hash_project_to:?} (projection baselines: hash + PCA; S1,S2 only)"
        )?;
        writeln!(out)?;
    }

    let mut gates = GateSummary::default();

    let common = CaseCommon {
        csv: args.csv,
        n,
        ksg_cfg: &ksg_cfg,
        hash_project_to,
    };
    for d in dims {
        for &seed in &seeds {
            for name in [
                "independent_additive",
                "redundant_copy",
                "unique_s1",
                "xor_like",
            ] {
                let res = run_case(out, common, CaseSpec { name, d, seed })?;
                gates.observe_case(name, d, res.metrics, res.diag);
            }
            if !common.csv {
                writeln!(out)?;
            }
        }
        if !common.csv {
            writeln!(out)?;
        }
    }

    if common.csv {
        writeln!(out)?;
        write_gaussian_csv_header(out)?;
    }
    run_gaussian_channel_strong_dependence_sweep(out, common.csv, 900, &ksg_cfg, 0x51A7_2026)?;

    if !args.csv {
        writeln!(out, "--- Experiment 0 Summary ---")?;
        gates.print(out)?;
    }

    if let Some(path) = args.summary_json.as_deref() {
        write_summary_json(path, &gates, n, k, &dims, &seeds, hash_project_to)?;
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

    if common.csv {
        write_case_csv_row(
            out,
            common.ksg_cfg,
            CaseCsvRow {
                name: spec.name,
                seed: spec.seed,
                projection: ProjectionMethod::None,
                d,
                n,
                project_to: None,
                metrics: baseline,
                diag,
            },
        )?;
    } else {
        print_metrics(out, spec.name, d, spec.seed, baseline)?;
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
            if common.csv {
                write_case_csv_row(
                    out,
                    common.ksg_cfg,
                    CaseCsvRow {
                        name: &case_name,
                        seed: spec.seed,
                        projection: ProjectionMethod::Hash,
                        d: dout,
                        n,
                        project_to: Some(dout),
                        metrics: projected,
                        diag: diag_p,
                    },
                )?;
            } else {
                print_metrics(out, &case_name, dout, spec.seed, projected)?;
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
            if common.csv {
                write_case_csv_row(
                    out,
                    common.ksg_cfg,
                    CaseCsvRow {
                        name: &case_name,
                        seed: spec.seed,
                        projection: ProjectionMethod::Pca,
                        d: dout,
                        n,
                        project_to: Some(dout),
                        metrics: projected,
                        diag: diag_p,
                    },
                )?;
            } else {
                print_metrics(out, &case_name, dout, spec.seed, projected)?;
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

    let dr_s1 = relative_delta(d.gromov_s1, d.diam_s1);
    let dr_s2 = relative_delta(d.gromov_s2, d.diam_s2);
    let dr_s12 = relative_delta(d.gromov_s12, d.diam_s12);
    let dr_t = relative_delta(d.gromov_t, d.diam_t);

    writeln!(
        out,
        "{:>20} {:>7} | d_rel(s1)={:>6.3} | d_rel(s2)={:>6.3} | d_rel(s1,s2)={:>6.3} | d_rel(t)={:>6.3}",
        "", "", dr_s1, dr_s2, dr_s12, dr_t
    )?;
    Ok(())
}

fn run_gaussian_channel_strong_dependence_sweep(
    out: &mut dyn Write,
    csv: bool,
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

    if !csv {
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
        if csv {
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
    if !csv {
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
    r_bar: f64,
    v_bar: f64,
    red_ehrlich: f64,
    red_local_min: f64,
    red_disjunction: f64,
    syn_ehrlich: f64,
}

#[derive(Debug, Default, Clone)]
struct GateSummary {
    case_results: usize,
    red_zero_checks: usize,
    red_zero_passes: usize,
    monotonicity_violations: usize,
    cmi_violations: usize,
    invariant_violations: usize,
    geometry_warnings: usize,
}

impl GateSummary {
    fn observe_case(&mut self, name: &str, d: usize, metrics: Metrics, diag: Diagnostics) {
        const TOL: f64 = 1e-9;
        self.case_results += 1;

        if metrics.mi_s1s2_t + TOL < metrics.mi_s1_t {
            self.monotonicity_violations += 1;
        }
        if metrics.mi_s1s2_t + TOL < metrics.mi_s2_t {
            self.monotonicity_violations += 1;
        }

        if metrics.mi_s1s2_t - metrics.mi_s2_t < -TOL {
            self.cmi_violations += 1;
        }
        if metrics.mi_s1s2_t - metrics.mi_s1_t < -TOL {
            self.cmi_violations += 1;
        }

        if !bounded_degree(metrics.r_bar, 0.0, 2.0, TOL)
            || !bounded_degree(metrics.v_bar, 0.0, 2.0, TOL)
        {
            self.invariant_violations += 1;
        }

        if name == "independent_additive" {
            self.red_zero_checks += 1;
            if metrics.red_ehrlich.abs() < red_zero_threshold(d) {
                self.red_zero_passes += 1;
            }
            let dr_s1 = relative_delta(diag.gromov_s1, diag.diam_s1);
            if diag.id_s1 > 20.0 || diag.dc_cv_s1 < 0.1 || dr_s1 < 0.1 {
                self.geometry_warnings += 1;
            }
        }
    }

    fn status(&self) -> &'static str {
        if self.case_results == 0 {
            return "NO-GO";
        }
        if self.monotonicity_violations == 0
            && self.cmi_violations == 0
            && self.invariant_violations == 0
            && self.geometry_warnings == 0
            && self.red_zero_checks == self.red_zero_passes
        {
            "GO"
        } else if self.red_zero_checks > 0
            && self.red_zero_passes * 2 >= self.red_zero_checks
            && self.invariant_violations == 0
        {
            "PIVOT"
        } else {
            "NO-GO"
        }
    }

    fn print(&self, out: &mut dyn Write) -> io::Result<()> {
        writeln!(
            out,
            "Passes (Independent Additive Zero-Redundancy check): {}/{}",
            self.red_zero_passes, self.red_zero_checks
        )?;
        writeln!(out, "Case Results: {}", self.case_results)?;
        writeln!(out, "Geometry Warnings: {}", self.geometry_warnings)?;
        writeln!(
            out,
            "Monotonicity Violations: {}",
            self.monotonicity_violations
        )?;
        writeln!(out, "CMI Nonnegativity Violations: {}", self.cmi_violations)?;
        writeln!(
            out,
            "Invariant Bound Violations: {}",
            self.invariant_violations
        )?;
        writeln!(out, "Status: {}", self.status())?;
        Ok(())
    }
}

fn bounded_degree(value: f64, lo: f64, hi: f64, tol: f64) -> bool {
    value.is_finite() && value >= lo - tol && value <= hi + tol
}

fn red_zero_threshold(d: usize) -> f64 {
    if d <= 10 {
        0.1
    } else if d <= 100 {
        0.2
    } else {
        0.3
    }
}

fn relative_delta(delta: f64, diameter: f64) -> f64 {
    if delta.is_finite() && diameter.is_finite() && diameter > 0.0 {
        2.0 * delta / diameter
    } else {
        f64::NAN
    }
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

    let r_bar = average_degree_of_redundancy(&[mi_s1_t, mi_s2_t], mi_s1s2_t);
    let v_bar = average_degree_of_vulnerability(mi_s1s2_t, &[mi_s2_t, mi_s1_t]);

    Ok(Metrics {
        mi_s1_t,
        mi_s2_t,
        mi_s1s2_t,
        ci,
        r_bar,
        v_bar,
        red_ehrlich,
        red_local_min,
        red_disjunction,
        syn_ehrlich: mi_s1s2_t - mi_s1_t - mi_s2_t + red_ehrlich,
    })
}

fn print_metrics(
    out: &mut dyn Write,
    name: &str,
    d: usize,
    seed: u64,
    m: Metrics,
) -> io::Result<()> {
    writeln!(
        out,
        "{name:>20} d={d:<4} seed={seed:<10} | I1={:>7.3} I2={:>7.3} I12={:>7.3} CI={:>7.3} | r_bar={:>5.2} v_bar={:>5.2} | Red(ehr)={:>7.3} Syn(ehr)={:>7.3} | Red(disj)={:>7.3}",
        m.mi_s1_t,
        m.mi_s2_t,
        m.mi_s1s2_t,
        m.ci,
        m.r_bar,
        m.v_bar,
        m.red_ehrlich,
        m.syn_ehrlich,
        m.red_disjunction,
    )?;
    Ok(())
}

fn write_case_csv_header(out: &mut dyn Write) -> io::Result<()> {
    writeln!(
        out,
        "case_name,seed,projection,d,n,k,metric,project_to,mi_s1_t,mi_s2_t,mi_s1s2_t,ci,r_bar,v_bar,red_ehrlich,red_local_min,red_disjunction,syn_ehrlich,id_s1,id_s2,id_t,id_s12,dc_cv_s1,dc_nnratio_s1,dc_cv_s2,dc_nnratio_s2,dc_cv_s12,dc_nnratio_s12,gromov_s1,gromov_s2,gromov_s12,gromov_t,dr_s1,dr_s2,dr_s12,dr_t"
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
    seed: u64,
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
    let dr_s1 = relative_delta(row.diag.gromov_s1, row.diag.diam_s1);
    let dr_s2 = relative_delta(row.diag.gromov_s2, row.diag.diam_s2);
    let dr_s12 = relative_delta(row.diag.gromov_s12, row.diag.diam_s12);
    let dr_t = relative_delta(row.diag.gromov_t, row.diag.diam_t);

    writeln!(
        out,
        "{},{},{},{},{},{},{:?},{project_to},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e}",
        row.name,
        row.seed,
        row.projection.as_str(),
        row.d,
        row.n,
        ksg_cfg.k,
        ksg_cfg.metric,
        row.metrics.mi_s1_t,
        row.metrics.mi_s2_t,
        row.metrics.mi_s1s2_t,
        row.metrics.ci,
        row.metrics.r_bar,
        row.metrics.v_bar,
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
