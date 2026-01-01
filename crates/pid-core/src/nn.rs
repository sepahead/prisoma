use crate::error::{PidError, PidResult};
use crate::matrix::MatRef;
use crate::metric::Metric;

/// Brute-force kNN radius in a joint space composed of multiple blocks.
///
/// Distance between two samples is defined as the maximum of the per-block distances.
/// With `Metric::Chebyshev`, this matches Chebyshev distance on the concatenated vector.
pub fn kth_neighbor_distance_joint_max(
    blocks: &[MatRef<'_>],
    i: usize,
    k: usize,
    metric: Metric,
) -> PidResult<f64> {
    if blocks.is_empty() {
        return Err(PidError::NotImplemented {
            feature: "kth_neighbor_distance_joint_max with empty blocks",
        });
    }

    let n = blocks[0].nrows();
    if k == 0 || n <= k {
        return Err(PidError::InvalidK { k, n_samples: n });
    }
    for b in blocks.iter().skip(1) {
        if b.nrows() != n {
            return Err(PidError::RowCountMismatch {
                context: "kth_neighbor_distance_joint_max",
                left_rows: n,
                right_rows: b.nrows(),
            });
        }
    }

    let mut dists = Vec::with_capacity(n - 1);
    for j in 0..n {
        if i == j {
            continue;
        }
        let mut dist = 0.0f64;
        for b in blocks {
            dist = dist.max(metric.distance(b.row(i), b.row(j)));
        }
        dists.push(dist);
    }

    let kth = k - 1;
    dists.select_nth_unstable_by(kth, |a, b| a.total_cmp(b));
    Ok(dists[kth])
}

/// Count neighbors of sample `i` within radius `eps` in a single space.
///
/// Uses strict inequality (`< eps`) to mirror typical KSG tie handling.
pub fn count_neighbors_within(m: MatRef<'_>, i: usize, eps: f64, metric: Metric) -> usize {
    let n = m.nrows();
    let mi = m.row(i);
    let mut count = 0usize;
    for j in 0..n {
        if i == j {
            continue;
        }
        if metric.distance(mi, m.row(j)) < eps {
            count += 1;
        }
    }
    count
}
