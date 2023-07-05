//! This module contains the reporter that is used to report the benchmark result.
//! Generally, we provide two reporters.
//!
//! [`SimpleBenchReporter`] is a simple reporter that collects the results of workers on a list.
//! [`DistributedBenchReporter`] will send the results to a remote reporter for aggregration.
//!
//! Example usage of [`SimpleBenchReporter`]:
//!
//! ```
//! #![feature(get_mut_unchecked)]

//! extern crate netbencher_core;
//!
//! use std::sync::Arc;
//!
//! use netbencher_core::{BenchRunner, SimpleBenchReporter};
//!
//! fn main() {    
//!     let mut runner = BenchRunner::new(1);
//!     runner.run(
//!         // The evaluated function will increase the statics per second
//!         |worker_id, runner, mut stats, _| {
//!             println!("Worker {} started", worker_id);
//!             while runner.running() {
//!                 std::thread::sleep(std::time::Duration::from_secs(1));
//!                 unsafe { Arc::get_mut_unchecked(&mut stats).finished_one_op() };
//!             }
//!         },
//!         (),
//!     );
//!
//!     let mut reporter = SimpleBenchReporter::new();
//!     for _ in 0..1 {
//!         std::thread::sleep(std::time::Duration::from_secs(1));
//!         let stat = runner.report(&mut reporter);
//!         println!("Results: {}", stat);
//!
//!     }
//!
//!     runner.stop().unwrap();
//! }
//!
//! ```
//!
use std::ops;
use std::sync::Arc;

use serde_derive::{Deserialize, Serialize};

mod simple_reporter;
pub use simple_reporter::SimpleBenchReporter;

mod coordinated_reporter;
pub use coordinated_reporter::{CoordinatedReporter, CoordinatedReporterMaster};



#[derive(Clone, Copy, Debug, PartialEq)]
struct AvgRdtsc {
    pub value: u64,
    pub cnt_num: i64,
}

/// BenchStat is a single stat that is reported by a worker
/// It records the following things:
/// > 1. num ops finished during this period
/// > 2. latency of each op
/// etc.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(align(128))]
pub struct BenchStat {
    /// The number of ops finished during this period
    pub num_ops_finished: u64,

    /// The average rdtsc value of ops reported
    avg_rdtsc: AvgRdtsc,
}

impl BenchStat {
    /// Reset the stat
    pub fn reset(&mut self) {
        self.num_ops_finished = 0;
        self.avg_rdtsc = AvgRdtsc {value: 0, cnt_num: 0};
    }

    /// Mark the stat that one op is finished
    pub fn finished_one_op(&mut self) {
        self.finished_batch_ops(1);
    }

    /// Mark the stat that a batch of ops are finished
    pub fn finished_batch_ops(&mut self, num_ops: u64) {
        self.num_ops_finished += num_ops;
    }

    /// Record the average rdtsc for each op
    pub fn record_avg_rdtsc(&mut self, num: i64) {
        self.avg_rdtsc.cnt_num += 1;
        self.avg_rdtsc.value += 
            ((num - self.avg_rdtsc.value as i64) /self.avg_rdtsc.cnt_num) as u64;
    }
}

impl Default for BenchStat {
    fn default() -> Self {
        Self {
            num_ops_finished: 0,
            avg_rdtsc: AvgRdtsc {value: 0, cnt_num: 0},
        }
    }
}

/// A collection of BenchStat to transform it to a user-readable format
/// Basically, we care about the following stuffs:
/// 1. throughput
/// 2. average latency
/// 3. 99th latency (TBD)
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct CollectedBenchStat {
    /// The number of ops finished during a period
    pub throughput: f64,
    /// The average latency of all ops
    pub avg_latency: f64,
    /// The 99th latency of all ops
    pub p99_latency: f64,

    /// The id of the stats
    pub id: usize,
}

impl Default for CollectedBenchStat {
    fn default() -> Self {
        Self {
            throughput: 0.0,
            avg_latency: 0.0,
            p99_latency: 0.0,
            id: 0,
        }
    }
}

impl CollectedBenchStat {
    /// Reset the stat
    pub fn reset(&mut self) {
        self.throughput = 0.0;
        self.avg_latency = 0.0;
        self.p99_latency = 0.0;
    }
}

/// BenchReporter is a trait that defines how to report stats collected.
pub trait BenchReporter {
    /// Collect the results from the list of BenchStats and collect it to a CollectedBenchStat,
    /// which is a user-readable format.
    fn report_collected_stat(&mut self, stats: &Vec<Arc<BenchStat>>) -> CollectedBenchStat;
}

/// AsyncBenchReporter is a trait that defines how to report stats collected.
/// The only difference with [`BenchmarkReporter`] is that it is async.
pub trait AsyncBenchReporter { 
    /// Collect the results from the list of BenchStats and collect it to a CollectedBenchStat,
    /// which is a user-readable format (async version).
    async fn async_report_collect_stat(&mut self, stats: &Vec<Arc<BenchStat>>) -> CollectedBenchStat;
}

impl ops::Add for CollectedBenchStat {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        // FIXME: the latency calculation is not so properly here
        Self {
            throughput: self.throughput + other.throughput,
            avg_latency: (self.avg_latency + other.avg_latency) / 2.0,
            p99_latency: (self.p99_latency + other.p99_latency) / 2.0,
            id: self.id,
        }
    }
}

impl ops::Add for BenchStat {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            num_ops_finished: self.num_ops_finished + other.num_ops_finished,
            avg_rdtsc: AvgRdtsc {value: 0, cnt_num: 0},
        }
    }
}

impl ops::Sub for BenchStat {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            num_ops_finished: self.num_ops_finished - other.num_ops_finished,
            avg_rdtsc: AvgRdtsc {value: 0, cnt_num: 0},
        }
    }
}

impl std::fmt::Display for CollectedBenchStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@{} Throughput: {:.4} Mops/s, Avg Latency: {:.2} µs",
            self.id, self.throughput, self.avg_latency
        )
    }
}
