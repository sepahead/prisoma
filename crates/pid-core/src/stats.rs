/// Digamma / psi function ψ(x).
///
/// Implementation: recurrence to shift into a "large x" regime + asymptotic expansion.
///
/// Units: natural logarithm (nats).
pub fn digamma(x: f64) -> f64 {
    debug_assert!(x.is_finite());
    debug_assert!(x > 0.0);

    let mut x = x;
    let mut acc = 0.0;

    // Recurrence for small x: ψ(x) = ψ(x+1) - 1/x
    while x < 6.0 {
        acc -= 1.0 / x;
        x += 1.0;
    }

    // Asymptotic series (Stirling-like).
    // ψ(x) ≈ ln(x) - 1/(2x) - 1/(12x²) + 1/(120x⁴) - 1/(252x⁶) + ...
    let inv = 1.0 / x;
    let inv2 = inv * inv;
    let inv4 = inv2 * inv2;
    let inv6 = inv4 * inv2;

    acc + x.ln() - 0.5 * inv - (1.0 / 12.0) * inv2 + (1.0 / 120.0) * inv4 - (1.0 / 252.0) * inv6
}
