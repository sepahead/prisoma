#![forbid(unsafe_code)]

mod ci;
mod distance_matrix;
mod error;
mod hierarchy;
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
pub use hierarchy::{
    hierarchical_pairwise, hierarchical_triplet, HierarchicalConfig, HierarchicalTriplet,
    PairSelection, PairwiseScreen,
};
pub use isx::{isx_redundancy, IsxConfig, IsxMethod};
pub use ksg::{ksg_local_mi_terms, ksg_mi, ksg_mi_concat_xy, KsgConfig, NegativeHandling};
pub use matrix::{concat_horiz, MatOwned, MatRef};
pub use metric::Metric;
pub use pid2::{pid2_isx, pid2_isx_estimate, Pid2Config, Pid2Estimate, Pid2Result};
pub use pid3::{pid3_isx, Antichain3, Pid3Atom, Pid3Config, Pid3Redundancy, Pid3Result};
pub use preprocess::{HashProjector, Jitter, Standardizer};
