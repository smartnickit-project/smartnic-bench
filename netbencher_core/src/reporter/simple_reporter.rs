use std::time::Instant;

use super::{BenchReporter, BenchStat, CollectedBenchStat};

/// A simple reporter that reports the throughput and latency of workers from this machine.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SimpleBenchReporter {
    stats_of_last_period: BenchStat,
    last_record_time: Instant,
    id: usize,
}

impl Default for SimpleBenchReporter {
    fn default() -> Self {
        Self {
            stats_of_last_period: BenchStat::default(),
            last_record_time: Instant::now(),
            id: 0,
        }
    }
}

impl SimpleBenchReporter {
    /// Create a new simple reporter
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new simple reporter with a given id
    pub fn new_with_id(id: usize) -> Self {
        Self {
            stats_of_last_period: BenchStat::default(),
            last_record_time: Instant::now(),
            id,
        }
    }
}

impl BenchReporter for SimpleBenchReporter {
    fn report_collected_stat(
        &mut self,
        stats: &Vec<std::sync::Arc<BenchStat>>,
    ) -> CollectedBenchStat {
        let mut new_stat = BenchStat::default();
        for stat in stats {
            new_stat.num_ops_finished += stat.num_ops_finished;
        }

        let now = Instant::now();
        let gap = new_stat - self.stats_of_last_period;

        // microseconds passed
        let duration = now.duration_since(self.last_record_time).as_micros() as f64;
        // mops
        let throughput = gap.num_ops_finished as f64 / duration;
        // microseconds
        let avg_latency = duration / gap.num_ops_finished as f64;

        self.stats_of_last_period = new_stat;
        self.last_record_time = now;

        CollectedBenchStat {
            id: self.id,
            throughput,
            avg_latency,
            p99_latency: 0.0,
        }
    }
}
