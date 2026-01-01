#![forbid(unsafe_code)]

mod ci;
mod error;
mod isx;
mod ksg;
mod matrix;
mod metric;
mod pid2;
mod stats;

pub use ci::co_information_pairwise;
pub use error::{PidError, PidResult};
pub use isx::{isx_redundancy, IsxConfig};
pub use ksg::{ksg_mi, KsgConfig, NegativeHandling};
pub use matrix::{concat_horiz, MatOwned, MatRef};
pub use metric::Metric;
pub use pid2::{pid2_isx, Pid2Config, Pid2Estimate, Pid2Result};
