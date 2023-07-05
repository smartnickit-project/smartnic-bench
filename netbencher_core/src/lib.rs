#![feature(get_mut_unchecked, async_fn_in_trait)]
//! Core code for bootstrap the benchmark
//!
//! The code represents a client process running on a client machine.
//! We use a framework to simplfiy bootstraping the tests.
//!
//! The framework ([`BenchRunner`]) will bootstrap a set of (pre-defined) threads to run user-specificed thread body functions.
//! The function is expceted to record some statistics ([`BenchStat`])]),
//! and the framework will automatically collect and report these numbers ([`CollectedBenchStat`]).
//!
//! The [`BenchReporter`] trait will implement various strategies to report the statistics,
//! e.g. print to stdout, write to a file, or send to a remote reporter.
//! Currently, we provide two reporter implementation:
//! - [`SimpleReporter`] will collect and print stats from threads on one server.
//! - [`CoordinatedReporter`] will send the stats to a remote reporter for aggregation.
//!
//! The simplest way is to use the [`BenchRunner::run`] function to run a function on `num_workers` threads:
//!
//! ```no_run
//! use netbencher_core::BenchRunner;
//!
//! let mut runner = BenchRunner::new(4);
//! runner.run(|thread_id, runner, stat, input| {
//!    // do something
//!    // mark the stats
//!    1 // return the result
//! }, 0);
//! runner.stop();
//!
//! ```
//!
//! One can also use a reporter to report the current states of the runner:
//!
//! ```no_run
//! use netbencher_core::BenchRunner;
//! use netbencher_core::SimpleBenchReporter;
//!
//! let mut runner = BenchRunner::new(4);
//! runner.run(|thread_id, runner, stat, input| {
//!     while runner.running() {
//!       // do something
//!       // mark the stats
//!    }
//!    1 // return the result
//! }, 0);
//!
//! let mut reporter = SimpleBenchReporter::new();
//! for _ in 0..10 {
//!     std::thread::sleep(std::time::Duration::from_secs(1));
//!     let stat = runner.report(&mut reporter);
//!     println!("Results: {}", stat);
//! }
//!
//! runner.stop();
//! ```
//!
//!
//! For more exmaples, please refer to the examples folder.
//!

#![deny(missing_docs)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Reporter will implement the BenchReporter related modules
mod reporter;
pub use reporter::{
    AsyncBenchReporter, BenchReporter, BenchStat, CollectedBenchStat, CoordinatedReporter,
    CoordinatedReporterMaster, SimpleBenchReporter,
};

/// Global control data structure to manage the bench workers
/// T : the return type of the worker
///
pub struct BenchRunner<T> {
    handlers: Vec<JoinHandle<T>>,
    worker_stats: Vec<Arc<BenchStat>>,
    num_workers: usize,
    running: AtomicBool,
}

impl<T> BenchRunner<T> {
    /// Create a new bench runner with a given number of workers
    pub fn new(num_workers: usize) -> Arc<Self> {
        Arc::new(Self {
            handlers: Vec::new(),
            worker_stats: Vec::new(),
            num_workers,
            running: AtomicBool::new(true),
        })
    }

    /// Run a given function on each worker
    ///
    /// The passsed in function is expected to take the signature of the following:
    ///
    /// fn worker(thread_id : usize, runner : Arc <BenchRunner<T>>, stat : Arc<BenchStat>, input : Input) -> T
    ///
    pub fn run<F, Input>(self: &mut Arc<Self>, func: F, input: Input)
    where
        F: FnOnce(usize, Arc<Self>, Arc<BenchStat>, Input) -> T + Send + 'static + Clone,
        T: Send + 'static + Sync + Copy,
        Input: Send + 'static + Sync + Clone,
    {
        let runner = self.clone();
        let self_mut = unsafe { Arc::get_mut_unchecked(self) };

        for i in 0..runner.num_workers {
            let inner_runner = runner.clone();
            let stat: Arc<BenchStat> = Arc::new(Default::default());
            self_mut.worker_stats.push(stat.clone());
            let input_args = input.clone();
            let func = func.clone();
            let handler = std::thread::spawn(move || func(i, inner_runner, stat, input_args));
            self_mut.handlers.push(handler);
        }
    }

    /// Stop all the workers
    pub fn stop(self: &mut Arc<Self>) -> std::thread::Result<Vec<T>> {
        let mut res = Vec::new();

        let self_mut = unsafe { Arc::get_mut_unchecked(self) };
        self_mut.running.store(false, Ordering::SeqCst);

        while !self_mut.handlers.is_empty() {
            res.push(self_mut.handlers.pop().unwrap().join()?);
        }

        Ok(res)
    }

    /// Report the collected stats from the managed workers
    pub fn report(self: &Self, reporter: &mut dyn BenchReporter) -> CollectedBenchStat {
        reporter.report_collected_stat(&self.worker_stats)
    }

    /// Report the collected stats from the managed workers (async version)
    /// Due to compile problem of async trait, currently I will fix the reporter to be CoordinatedReporter
    pub async fn report_async<R: BenchReporter>(
        self: &Self,
        reporter: &mut CoordinatedReporter<R>,
    ) -> CollectedBenchStat {
        reporter.async_report_collect_stat(&self.worker_stats).await
    }

    /// Check if the runner is still running
    #[inline]
    pub fn running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

mod tests {
    #[test]
    fn test_simple_runner() {
        let mut runner = super::BenchRunner::new(2);
        runner.run(
            |_, r, _, _| {
                while r.running() {
                    println!("Hello world");
                }
            },
            (),
        );
        runner.stop().unwrap();
    }

    #[test]
    fn test_runner_stop() {
        let mut runner = super::BenchRunner::new(2);
        runner.run(
            |_, r, _, _| {
                println!("Hello world");
                while r.running() {
                    std::thread::yield_now();
                }
            },
            (),
        );

        runner.stop().unwrap();
    }

    #[test]
    fn test_runner_input_work() {
        let mut runner = super::BenchRunner::new(2);
        let input: usize = 73;

        runner.run(
            |_, r, _, input| {
                println!("Hello world");
                while r.running() {
                    std::thread::yield_now();
                }
                assert_eq!(input, 73);
            },
            input,
        );

        runner.stop().unwrap();
    }

    #[test]
    fn test_runner_output_work() {
        let mut runner = super::BenchRunner::new(10);
        let input: usize = 73;

        runner.run(
            |thread_id, r, _, input| {
                println!("Hello world");
                while r.running() {
                    std::thread::yield_now();
                }
                assert_eq!(input, 73);
                thread_id
            },
            input,
        );

        let res = runner.stop().unwrap();

        let mut sum = 0;
        for i in res {
            sum += i;
        }
        assert_eq!(sum, 0 + 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9);
    }
}
