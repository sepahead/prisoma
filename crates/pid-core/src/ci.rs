use crate::error::PidResult;
use crate::ksg::{ksg_mi, ksg_mi_concat_xy, ksg_mi_xblocks, KsgConfig};
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

/// 3-source co-information (interaction information / Shannon invariant) computed via KSG MI estimates:
///
/// CI(X,Y,Z;T) = I(X;T)+I(Y;T)+I(Z;T)
///              - I(X,Y;T) - I(X,Z;T) - I(Y,Z;T)
///              + I(X,Y,Z;T)
///
/// Sign convention used in `grandplan.md`: negative CI indicates net synergy.
pub fn co_information_triplet(
    x: MatRef<'_>,
    y: MatRef<'_>,
    z: MatRef<'_>,
    t: MatRef<'_>,
    cfg: &KsgConfig,
) -> PidResult<f64> {
    let i_xt = ksg_mi(x, t, cfg)?;
    let i_yt = ksg_mi(y, t, cfg)?;
    let i_zt = ksg_mi(z, t, cfg)?;

    let i_xyt = ksg_mi_concat_xy(x, y, t, cfg)?;
    let i_xzt = ksg_mi_concat_xy(x, z, t, cfg)?;
    let i_yzt = ksg_mi_concat_xy(y, z, t, cfg)?;

    let i_xyzt = ksg_mi_xblocks(&[x, y, z], t, cfg)?;

    Ok(i_xt + i_yt + i_zt - i_xyt - i_xzt - i_yzt + i_xyzt)
}
