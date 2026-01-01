use pid_core::{co_information_pairwise, ksg_mi, KsgConfig, MatRef, NegativeHandling};

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
        // Uniform in [0,1).
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

#[test]
fn ksg_mi_is_small_for_independent_uniforms() {
    let mut rng = Rng64::new(42);
    let n = 250;
    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);
    for _ in 0..n {
        x.push(rng.next_f64());
        y.push(rng.next_f64());
    }

    let x = MatRef::new(&x, n, 1).unwrap();
    let y = MatRef::new(&y, n, 1).unwrap();

    let cfg = KsgConfig {
        k: 3,
        negative_handling: NegativeHandling::Allow,
        ..Default::default()
    };
    let mi = ksg_mi(x, y, &cfg).unwrap();

    assert!(mi.is_finite());
    assert!(mi.abs() < 0.6, "expected near-0 MI, got {mi}");
}

#[test]
fn ksg_mi_is_larger_for_noisy_copy() {
    let mut rng = Rng64::new(123);
    let n = 300;
    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);
    for _ in 0..n {
        let xi = rng.next_f64();
        let yi = xi + 0.05 * rng.normal();
        x.push(xi);
        y.push(yi);
    }

    let x = MatRef::new(&x, n, 1).unwrap();
    let y = MatRef::new(&y, n, 1).unwrap();

    let cfg = KsgConfig {
        k: 3,
        negative_handling: NegativeHandling::Allow,
        ..Default::default()
    };
    let mi = ksg_mi(x, y, &cfg).unwrap();

    assert!(mi.is_finite());
    assert!(mi > 0.5, "expected MI > 0.5 nats, got {mi}");
}

#[test]
fn exp0_co_information_smoke() {
    // Minimal Experiment 0-ish smoke: CI is finite.
    let mut rng = Rng64::new(999);
    let n = 250;
    let mut s1 = Vec::with_capacity(n);
    let mut s2 = Vec::with_capacity(n);
    let mut t = Vec::with_capacity(n);
    for _ in 0..n {
        let a = rng.next_f64();
        let b = rng.next_f64();
        let noise = 0.01 * rng.normal();
        s1.push(a);
        s2.push(b);
        t.push(a + b + noise);
    }

    let s1 = MatRef::new(&s1, n, 1).unwrap();
    let s2 = MatRef::new(&s2, n, 1).unwrap();
    let t = MatRef::new(&t, n, 1).unwrap();

    let cfg = KsgConfig::default();
    let ci = co_information_pairwise(s1, s2, t, &cfg).unwrap();
    assert!(ci.is_finite());
}
