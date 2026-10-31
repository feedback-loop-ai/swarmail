//! Report arithmetic (spec R3): nearest-rank quantiles over whatever
//! succeeded, censored counts beside them, and the two throughput windows.
//! Nothing here clamps, drops or takes an absolute value.

use serde::Serialize;

/// Nearest-rank percentile: the smallest sample with at least `p`% of the
/// samples at or below it — rank `ceil(p·n/100)`, 1-based. An empty sample
/// set has no quantile (null in the report), never zero.
pub fn nearest_rank(sorted: &[i64], p: u32) -> Option<i64> {
    if sorted.is_empty() || p == 0 || p > 100 {
        return None;
    }
    let n = sorted.len() as u64;
    let rank = (u64::from(p) * n).div_ceil(100).max(1);
    Some(sorted[(rank - 1) as usize])
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Quantiles {
    pub samples: usize,
    pub censored: usize,
    /// `(percentile, value_ns)`; the value is null when nothing succeeded.
    pub values: Vec<(u32, Option<i64>)>,
}

pub fn quantiles(mut values: Vec<i64>, censored: usize, percentiles: &[u32]) -> Quantiles {
    values.sort_unstable();
    Quantiles {
        samples: values.len(),
        censored,
        values: percentiles
            .iter()
            .map(|p| (*p, nearest_rank(&values, *p)))
            .collect(),
    }
}

/// One throughput repetition's reconciliation. Every attempted message
/// lands in exactly one disposition; every accepted one is either verified
/// visible once, duplicated, or missing.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Reconciliation {
    pub attempted: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub failed_before_terminator: u64,
    pub ambiguous: u64,
    /// Distinct accepted messages observed on the server.
    pub visible_unique: u64,
    pub duplicates: u64,
    pub missing_accepted: u64,
    /// Visible ids that were not accepted — an ambiguous send the server
    /// did keep, or something foreign; either way it is not a success.
    pub unexpected_visible: u64,
    pub fidelity_mismatches: u64,
    pub send_window_ns: u64,
    pub verified_window_ns: u64,
}

impl Reconciliation {
    /// Every attempt is in exactly one disposition.
    pub fn balanced(&self) -> bool {
        self.accepted + self.rejected + self.failed_before_terminator + self.ambiguous
            == self.attempted
    }

    /// The exact-count verdict: everything accepted, seen once, intact.
    pub fn exact(&self) -> bool {
        self.balanced()
            && self.accepted == self.attempted
            && self.visible_unique == self.accepted
            && self.duplicates == 0
            && self.missing_accepted == 0
            && self.unexpected_visible == 0
            && self.fidelity_mismatches == 0
    }

    /// Accepted messages per second of the timed send window.
    pub fn accepted_per_second(&self) -> Option<f64> {
        per_second(self.accepted, self.send_window_ns)
    }

    /// Accepted-and-observed messages per second of the window that runs
    /// from the first send through verification completion.
    pub fn verified_per_second(&self) -> Option<f64> {
        per_second(self.visible_unique, self.verified_window_ns)
    }
}

fn per_second(count: u64, window_ns: u64) -> Option<f64> {
    (window_ns > 0).then(|| count as f64 * 1e9 / window_ns as f64)
}
