use pid_core::{
    co_information_pairwise, concat_horiz, isx_redundancy, ksg_mi, IsxConfig, IsxMethod, KsgConfig,
    HashProjector, MatRef, Metric, NegativeHandling, Pid2Config, Standardizer,
};

fn main() {
    // Minimal Experiment 0 runner (Rust-side).
    //
    // This is intentionally small and brute-force; it exists to exercise the estimators end-to-end
    // on synthetic systems and to provide a place to iterate while building the full harness.

    let n = 250usize;
    let k = 3usize;
    let dims = [1usize, 10, 50, 256];
    let hash_project_to = Some(64usize);

    let ksg_cfg = KsgConfig {
        k,
        metric: Metric::Chebyshev,
        tie_epsilon: 1e-15,
        negative_handling: NegativeHandling::ClampToZero,
    };

    println!("Experiment 0 (Rust quick run)");
    println!("n={n}, k={k}, dims={dims:?}");
    println!("hash_project_to={hash_project_to:?} (feature hashing / CountSketch; S1,S2 only)");
    println!();

    for d in dims {
        run_case(
            "independent_additive",
            d,
            n,
            &ksg_cfg,
            42,
            hash_project_to,
        );
        run_case("redundant_copy", d, n, &ksg_cfg, 43, hash_project_to);
        run_case("unique_s1", d, n, &ksg_cfg, 44, hash_project_to);
        run_case("xor_like", d, n, &ksg_cfg, 45, hash_project_to);
        println!();
    }
}

fn run_case(
    name: &str,
    d: usize,
    n: usize,
    ksg_cfg: &KsgConfig,
    seed: u64,
    hash_project_to: Option<usize>,
) {
    let noise_std = 0.05;
    let (s1, s2, t) = match name {
        "independent_additive" => gen_independent_additive(n, d, noise_std, seed),
        "redundant_copy" => gen_redundant_copy(n, d, noise_std, seed),
        "unique_s1" => gen_unique_s1(n, d, noise_std, seed),
        "xor_like" => gen_xor_like(n, d, noise_std, seed),
        _ => unreachable!("unknown case: {name}"),
    };

    let s1 = MatRef::new(&s1, n, d).unwrap();
    let s2 = MatRef::new(&s2, n, d).unwrap();
    let t = MatRef::new(&t, n, 1).unwrap();

    let (s1z, _) = Standardizer::fit_transform(s1).unwrap();
    let (s2z, _) = Standardizer::fit_transform(s2).unwrap();
    let (tz, _) = Standardizer::fit_transform(t).unwrap();

    let baseline = compute_metrics(s1z.as_ref(), s2z.as_ref(), tz.as_ref(), ksg_cfg);

    print_metrics(name, d, baseline);

    if let Some(dout) = hash_project_to {
        if d > dout {
            let p1 = HashProjector::new(d, dout, 0xA11CE_u64 ^ seed).unwrap();
            let p2 = HashProjector::new(d, dout, 0xB22CE_u64 ^ seed).unwrap();

            let s1p = p1.transform(s1z.as_ref()).unwrap();
            let s2p = p2.transform(s2z.as_ref()).unwrap();

            // Re-standardize after projection so Chebyshev distance has comparable scale.
            let (s1p, _) = Standardizer::fit_transform(s1p.as_ref()).unwrap();
            let (s2p, _) = Standardizer::fit_transform(s2p.as_ref()).unwrap();

            let projected = compute_metrics(s1p.as_ref(), s2p.as_ref(), tz.as_ref(), ksg_cfg);
            print_metrics(&format!("{name}_hashproj"), dout, projected);
        }
    }
}

#[derive(Clone, Copy)]
struct Metrics {
    mi_s1_t: f64,
    mi_s2_t: f64,
    mi_s1s2_t: f64,
    ci: f64,
    red_sketch: f64,
    red_local_min: f64,
    red_disjunction: f64,
    syn_local_min: f64,
}

fn compute_metrics(s1: MatRef<'_>, s2: MatRef<'_>, t: MatRef<'_>, ksg_cfg: &KsgConfig) -> Metrics {
    let mi_s1_t = ksg_mi(s1, t, ksg_cfg).unwrap();
    let mi_s2_t = ksg_mi(s2, t, ksg_cfg).unwrap();
    let s1s2 = concat_horiz(s1, s2).unwrap();
    let mi_s1s2_t = ksg_mi(s1s2.as_ref(), t, ksg_cfg).unwrap();
    let ci = co_information_pairwise(s1, s2, t, ksg_cfg).unwrap();

    let red_sketch = isx_redundancy(
        s1,
        s2,
        t,
        &IsxConfig {
            k: ksg_cfg.k,
            metric: ksg_cfg.metric,
            tie_epsilon: ksg_cfg.tie_epsilon,
            method: IsxMethod::GrandplanSketch,
        },
    )
    .unwrap();

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
    )
    .unwrap();

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

    let pid_local_min = pid_core::pid2_isx(
        s1,
        s2,
        t,
        &Pid2Config {
            ksg: ksg_cfg.clone(),
            isx: IsxConfig {
                k: ksg_cfg.k,
                metric: ksg_cfg.metric,
                tie_epsilon: ksg_cfg.tie_epsilon,
                method: IsxMethod::LocalMinKsg,
            },
        },
    )
    .unwrap();

    Metrics {
        mi_s1_t,
        mi_s2_t,
        mi_s1s2_t,
        ci,
        red_sketch,
        red_local_min,
        red_disjunction,
        syn_local_min: pid_local_min.synergy,
    }
}

fn print_metrics(name: &str, d: usize, m: Metrics) {
    println!(
        "{name:>20} d={d:<4} | I1={:>7.3} I2={:>7.3} I12={:>7.3} CI={:>7.3} | Red(sketch)={:>7.3} Red(local_min)={:>7.3} Red(disj)={:>7.3} | Syn(local_min)={:>7.3}",
        m.mi_s1_t,
        m.mi_s2_t,
        m.mi_s1s2_t,
        m.ci,
        m.red_sketch,
        m.red_local_min,
        m.red_disjunction,
        m.syn_local_min,
    );
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
