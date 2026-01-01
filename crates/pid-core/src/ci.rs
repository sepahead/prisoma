use crate::error::PidResult;
use crate::ksg::{ksg_mi, ksg_mi_concat_xy, KsgConfig};
use crate::matrix::MatRef;

/// Pairwise co-information (a Shannon invariant) computed via KSG MI estimates:
///
/// CI(X,Y;T) = I(X;T) + I(Y;T) - I((X,Y);T)
///
/// Sign convention used in `grandplan.md`: negative CI indicates synergy.
pub fn co_information_pairwise(
    x: MatRef<'_>,
    y: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &KsgConfig,
) -> PidResult<f64> {
    let i_xt = ksg_mi(x, t, cfg)?;
    let i_yt = ksg_mi(y, t, cfg)?;
    let i_xyt = ksg_mi_concat_xy(x, y, t, cfg)?;
    Ok(i_xt + i_yt - i_xyt)
}
