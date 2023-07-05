mod client_construct;
pub use client_construct::{ perform_client_routine, recv_doca_config };

mod server_construct;
pub use server_construct::perform_server_routine;

mod connection;
pub use connection::{ DocaConnInfo, DocaConnInfoMsg };
pub use connection::DOCA_MAX_CONN_LENGTH;

use std::{ thread, time };
use std::time::Duration;
use tokio::runtime::Runtime;

use bench_util::doca::args::*;
use bench_util::{ MAX_CLIENTS, MIN_SERVER_LIFE };

use netbencher_core::{
    CoordinatedReporterMaster,
    BenchRunner,
    SimpleBenchReporter,
    CoordinatedReporter,
};

use log::*;

pub fn bootstrap_client(mut args: CmdlineArgs) {
    /* load config using TCP channel */
    let doca_conn_msg = Runtime::new().unwrap().block_on(recv_doca_config(args.listen_addr.parse().unwrap()));
    let mut runner = BenchRunner::new(args.threads as usize);
    // let mut runner = BenchRunner::new(args.threads.try_into().unwrap());
    runner.run(move |thread_id, runner, stat, args| {
        perform_client_routine(thread_id, runner, stat, doca_conn_msg.clone(), args);
    }, args.clone());

    let mut inner_reporter = SimpleBenchReporter::new_with_id(args.client_id.try_into().unwrap());

    for epoch in 0..args.life {
        thread::sleep(time::Duration::from_secs(1));
        info!("{}", runner.report(&mut inner_reporter));
    }
    runner.stop().unwrap();
}

pub fn bootstrap_server(mut args: CmdlineArgs) {
    if args.life < MIN_SERVER_LIFE {
        args.life = MIN_SERVER_LIFE;
    }

    let mut runner = BenchRunner::new(1);
    runner.run(|thread_id, runner, stat, args| {
        perform_server_routine(runner, args);
    }, args.clone());
    thread::sleep(Duration::from_secs(args.life.into()));
    runner.stop().unwrap();
}