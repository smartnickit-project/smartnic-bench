//! A reporter that reports the throughput and latency of workers from this machine.
//! This contains two parts: a master and several reporters.
//! The master collects the reports from the reporters and aggregates them accordingly, see `report_event_loop`.
//!
//! To use this module, first start the master at one node, then start the reporters at other nodes.
//! More specifically, see the following master example:
//!
//! ```ignore
//! let mut master = CoordinatedReporterMaster::new(
//!             "127.0.0.1:8888".parse().unwrap(),
//!         )
//!         .await
//!         .expect("failed to create the master");
//!
//!         master
//!             .report_event_loop(
//!                 Duration::from_secs(10), // run 10 seconds
//!                 Duration::from_secs(1),  // report every 1 seconds
//!            )
//!             .await
//!             .expect("Event loop report error");
//! ```
//!
//! Then, start any reporters at other nodes, for example:
//!
//! ```ignore
//! let bench = BenchRunner::new(1); // a bench runner w/ 1 threads
//!
//! let inner_reporter = SimpleBenchReporter::new_with_id(0); // can be any reporter
//! let mut reporter = CoordinatedReporter::new(
//!     "127.0.0.1:8888", inner_reporter).await.expect("failed to create the reporter");
//!
//! // send a report to the master
//! bench.async_report(&mut reporter).await;
//!
//! ```
//!
//! More example can be found at `netbencher-core/examples/coordinator_report_worker.rs` and `netbencher-core/examples/coordinator_report_master.rs`.
//!
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use tokio::net::UdpSocket;

use crate::AsyncBenchReporter;

use super::{BenchReporter, BenchStat, CollectedBenchStat};

use log::{info};

/// A coordinator that collects reports from [`CoordinatedReporter`]s.
///
/// # Note
/// We assume that `CoordinatedReporter`s IDs are continous and start from 0.
///
pub struct CoordinatedReporterMaster {
    num_reports: Vec<CollectedBenchStat>,
    record_time: Vec<Instant>,
    master_socket: UdpSocket,
}

impl CoordinatedReporterMaster {
    /// Create a new coordinated reporter master (async version)
    pub async fn new(num_reporters: usize, sock: SocketAddr) -> std::io::Result<Self> {
        let master_socket = UdpSocket::bind(sock).await?;
        let mut num_reports = Vec::with_capacity(num_reporters);
        let mut record_time = Vec::with_capacity(num_reporters);

        let now = Instant::now();
        for _ in 0..num_reporters {
            num_reports.push(Default::default());
            record_time.push(now);
        }

        Ok(Self {
            num_reports: num_reports,
            record_time,
            master_socket,
        })
    }

    /// Run an event loop to collect the reports from the reporters
    pub async fn report_event_loop(
        &mut self,
        duration: Duration,
        report_duration: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let mut tick_time = Instant::now();
        let mut buf = [0u8; 1024];

        let mut cur_time = Instant::now();

        while cur_time.duration_since(start_time) <= duration {
            // recv message
            match self.master_socket.try_recv_from(&mut buf) {
                Ok((n, _addr)) => {
                    let stat: CollectedBenchStat = serde_json::from_slice(&buf[..n])?;
                    self.num_reports[stat.id] = stat;
                    self.record_time[stat.id] = cur_time;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // continue;
                }
                Err(e) => {
                    return Err(Box::new(e));
                }
            }

            if cur_time.duration_since(tick_time) >= report_duration {
                let res = self.aggregrate_stats(&cur_time);
                info!("reports: {}", res);
                tick_time = Instant::now();
            }
            cur_time = Instant::now();
        }
        Ok(())
    }

    fn aggregrate_stats(&self, cur_time: &Instant) -> CollectedBenchStat {
        let mut res = CollectedBenchStat::default();
        for i in 0..self.num_reports.len() {
            // we will filter out outdated reports
            if cur_time.duration_since(self.record_time[i]) <= Duration::from_millis(1500) {
                res = res + self.num_reports[i];
            }
        }
        res
    }
}

/// A reporter that reports the throughput and latency of workers from this machine.
pub struct CoordinatedReporter<R: BenchReporter> {
    // We leverage the inner reporter to collect stats
    inner: R,
    master_addr: SocketAddr,
    master_socket: UdpSocket,
}

impl<R> CoordinatedReporter<R>
where
    R: BenchReporter,
{
    /// Create a new coordinated reporter (async version) using a known reporter
    pub async fn new(master_addr: SocketAddr, reporter: R) -> std::io::Result<Self> {
        let master_socket = UdpSocket::bind("0.0.0.0:0").await?;
        Ok(Self {
            inner: reporter,
            master_addr,
            master_socket,
        })
    }
}

impl<R> BenchReporter for CoordinatedReporter<R>
where
    R: BenchReporter,
{
    // 1. collect the stats
    // 2. send the stats to the master
    fn report_collected_stat(&mut self, stats: &Vec<Arc<BenchStat>>) -> CollectedBenchStat {
        self.inner.report_collected_stat(stats)
    }
}

impl<R> AsyncBenchReporter for CoordinatedReporter<R>
where
    R: BenchReporter,
{
    async fn async_report_collect_stat(
        &mut self,
        stats: &Vec<Arc<BenchStat>>,
    ) -> CollectedBenchStat {
        let res = self.inner.report_collected_stat(stats);
        self.master_socket
            .send_to(
                serde_json::to_vec(&res).unwrap().as_slice(),
                self.master_addr,
            )
            .await
            .expect("send UDP message to master failed");
        res
    }
}

impl<R> std::fmt::Display for CoordinatedReporter<R>
where
    R: BenchReporter,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CoordinatedReporter: report to {}", self.master_addr)
    }
}

mod tests {

    #[test]
    fn test_coordinated_reporter_create() {
        use super::*;
        use crate::SimpleBenchReporter;
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let r = SimpleBenchReporter::new_with_id(0);
            // the addr is not important here, can be any addr
            let _ = CoordinatedReporter::new("127.0.0.1:8080".parse().unwrap(), r);
        });
    }
}
