//! This module encapsulates the bootstrap functions for the one-sided RDMA benchmark. It provides functions to bootstrap the client and server sides of the benchmark. The client functions are encapsulated in `client_construct`
//! module and the server functions are encapsulated in `server_construct` module.
//! The module also defines constants for the maximum number of clients and minimum server life. The `bootstrap_client` function is used to bootstrap
//! the client side of the benchmark and the `bootstrap_server` function is used to bootstrap the server side of the benchmark. The module also imports necessary dependencies and defines a `BenchRunner` struct to run the benchmark.
//! Example of using in bench code:
//! ```rust
//! let server = true;
//! if server {
//!     bootstrap::bootstrap_server();
//! } else {
//!     bootstrap::bootstrap_client();
//! }
//!```

mod client_construct;
pub use client_construct::{
    perform_client_routine,
    perform_client_doorbell_routine,
    perform_client_signaled_routine,
    perform_client_doorbell_signaled_routine,
};

mod server_construct;
pub use server_construct::perform_server_routine;

use std::{ thread, time };
use std::time::Duration;
use tokio::runtime::Runtime;

use bench_util::args::*;
use bench_util::*;

use netbencher_core::{
    CoordinatedReporterMaster,
    BenchRunner,
    SimpleBenchReporter,
    CoordinatedReporter,
};

use log::*;

// Client bootstrap function
pub fn bootstrap_client(args: CmdlineArgs) {
    let mut runner = BenchRunner::new(args.threads.try_into().unwrap());
    runner.run(|thread_id, runner, stat, args| {
        match (args.doorbell, args.signaled) {
            (false, false) => {
                perform_client_routine(thread_id, runner, stat, args);
            }
            (true, false) => {
                info!("features: doorbell");
                perform_client_doorbell_routine(thread_id, runner, stat, args);
            }
            (false, true) => {
                info!("features: signaled");
                perform_client_signaled_routine(thread_id, runner, stat, args);
            }
            (true, true) => {
                info!("features: doorbell,signaled");
                perform_client_doorbell_signaled_routine(thread_id, runner, stat, args);
            }
        }
    }, args.clone());

    let mut inner_reporter = SimpleBenchReporter::new_with_id(args.client_id.try_into().unwrap());

    if args.report {
        Runtime::new()
            .unwrap()
            .block_on(async {
                let mut reporter = CoordinatedReporter::new(
                    args.report_addr.parse().unwrap(),
                    inner_reporter
                ).await.expect("failed to create the reporter");

                // send a report to the master
                for epoch in 0..args.life {
                    thread::sleep(time::Duration::from_secs(1));
                    runner.report_async(&mut reporter).await;
                }
            });
    } else {
        for epoch in 0..args.life {
            thread::sleep(time::Duration::from_secs(1));
            info!("{}", runner.report(&mut inner_reporter));
        }
    }
    runner.stop().unwrap();
}

// Server bootstrap function
pub fn bootstrap_server(mut args: CmdlineArgs) {
    if args.life < MIN_SERVER_LIFE {
        args.life = MIN_SERVER_LIFE;
    }

    let mut runner = BenchRunner::new(1);
    runner.run(|thread_id, runner, stat, args| { perform_server_routine(runner, args); }, args.clone());
    
    if args.report {
        Runtime::new()
            .unwrap()
            .block_on(async {
                let mut master = CoordinatedReporterMaster::new(
                    MAX_CLIENTS,
                    args.report_addr.parse().unwrap()
                ).await.expect("failed to create the master");

                master
                    .report_event_loop(
                        Duration::from_secs(args.life.into()),
                        Duration::from_secs(1)
                    ).await
                    .expect("event loop report error");
            });
    } else {
        thread::sleep(Duration::from_secs(args.life.into()));
    }
    runner.stop().unwrap();
}