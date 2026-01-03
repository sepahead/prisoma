//! `pid-core`: continuous mutual information + shared-exclusions PID (`I^sx_∩`) estimators.
//!
//! This crate implements:
//! - KSG mutual information (Kraskov et al. 2004) for continuous variables
//! - Wibral-group shared-exclusions redundancy `I^sx_∩(S1,S2;T)` (Makkeh et al. 2021)
//! - Continuous shared-exclusions estimator (Ehrlich et al. 2024)
//! - 2-source PID atoms derived from MI + `I^sx_∩`, and an optional 3-source SxPID
//! - A hierarchical “fast→slow” screening path for many-source settings
//!
//! # Units
//! All information quantities are reported in **nats** (natural logarithm).
//!
//! # Scientific contract
//! The mathematical object of interest is `I^sx_∩` and its derived PID atoms. Estimators are
//! finite-sample algorithms with failure modes; do not interpret downstream VLA results without
//! passing the Experiment 0 validation gate described in `grandplan.md`.
//!
//! # Estimator cautions (read before using on VLA embeddings)
//! - kNN estimators assume i.i.d. samples; trajectories violate this unless you subsample.
//! - High ambient/intrinsic dimension can collapse kNN geometry (distance concentration).
//! - Strong dependence (near-deterministic mappings) can require prohibitive samples even at low
//!   dimension.
//! - `I^sx_∩` (and PID atoms) are **not guaranteed non-negative** under all desiderata; negative
//!   values are possible and must be representable.
#![forbid(unsafe_code)]

mod ci;
mod distance_matrix;
mod error;
mod geometry;
mod hyperbolic;
mod hierarchy;
mod invariants;
mod isx;
mod ksg;
mod matrix;
mod metric;
mod nn;
mod pid2;
mod pid3;
mod preprocess;
mod stats;

pub use ci::{co_information_pairwise, co_information_triplet};
pub use distance_matrix::{symmetric_distances, SymmetricDistanceMatrix};
pub use error::{PidError, PidResult};
pub use geometry::{
    distance_concentration_stats, gromov_hyperbolicity, intrinsic_dimension_levina_bickel,
    DistanceConcentrationConfig, DistanceConcentrationStats, HyperbolicityConfig,
    IntrinsicDimConfig,
};
pub use hyperbolic::{hyperbolic_distance_lorentz, lorentz_dot, poincare_to_lorentz};
pub use hierarchy::{
    hierarchical_pairwise, hierarchical_triplet, HierarchicalConfig, HierarchicalTriplet,
    PairSelection, PairwiseScreen,
};
pub use invariants::{
    co_information_pairwise_discrete, entropy_discrete, joint_entropy_discrete,
    o_information_discrete, red_degree_discrete, vul_degree_discrete,
};
pub use isx::{isx_redundancy, IsxConfig, IsxMethod};
pub use ksg::{ksg_local_mi_terms, ksg_mi, ksg_mi_concat_xy, KsgConfig, NegativeHandling};
pub use matrix::{concat_horiz, MatOwned, MatRef};
pub use metric::Metric;
pub use pid2::{pid2_isx, pid2_isx_estimate, Pid2Config, Pid2Estimate, Pid2Result};
pub use pid3::{pid3_isx, Antichain3, Pid3Atom, Pid3Config, Pid3Redundancy, Pid3Result};
pub use preprocess::{HashProjector, Jitter, PcaProjector, Standardizer};
