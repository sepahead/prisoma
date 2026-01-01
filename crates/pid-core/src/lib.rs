#![forbid(unsafe_code)]

mod ci;
mod error;
mod isx;
mod ksg;
mod matrix;
mod metric;
mod nn;
mod pid2;
mod preprocess;
mod stats;

pub use ci::co_information_pairwise;
pub use error::{PidError, PidResult};
pub use isx::{isx_redundancy, IsxConfig, IsxMethod};
pub use ksg::{ksg_local_mi_terms, ksg_mi, KsgConfig, NegativeHandling};
pub use matrix::{concat_horiz, MatOwned, MatRef};
pub use metric::Metric;
pub use pid2::{pid2_isx, pid2_isx_estimate, Pid2Config, Pid2Estimate, Pid2Result};
pub use preprocess::{HashProjector, Jitter, Standardizer};
