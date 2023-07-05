#![feature(get_mut_unchecked)]

extern crate netbencher_core;

use clap::{Command, Arg, arg};
use simplelog::*;
use log::{info, warn, LevelFilter};
use std::sync::Arc;
use tokio::runtime::Runtime;

use netbencher_core::{BenchRunner, CoordinatedReporter, SimpleBenchReporter};

fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    ).unwrap();
    warn!("The master must alive before start the client");

    let matches = Command::new("bench example")
        .arg(
            Arg::new("num_workers")
                .short('n')
                .long("num_workers")
                .default_value("1"),
        )
        .arg(
            Arg::new("reporter_addr")
                .short('r')
                .long("reporter_addr")
                .required(true),
        )
        .arg(
            Arg::new("id")
                .short('i')
                .long("id")
                .default_value("0"),
        )
        .get_matches();

    let mut runner = BenchRunner::new(
        *matches.get_one("num_workers")
                .expect("failed to get num workers"),
    );
    runner.run(
        // The evaluated function will increase the statics per second
        |worker_id, runner, mut stats, _| {
            info!("Worker {} started", worker_id);
            while runner.running() {
                std::thread::sleep(std::time::Duration::from_secs(1));
                unsafe { Arc::get_mut_unchecked(&mut stats).finished_one_op() };
            }
        },
        (),
    );

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let inner_reporter =
            SimpleBenchReporter::new_with_id(*matches.get_one("id").unwrap());
        let mut reporter = CoordinatedReporter::new(
            matches.get_one::<String>("reporter_addr")
                .expect("failed to get the reporter_addr")
                .to_string().parse().unwrap(),
            inner_reporter,
        )
        .await
        .expect("failed to create the reporter");

        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let stat = runner.report_async(&mut reporter).await;
            // println!("Results: {}", stat);
        }
    });

    runner.stop().unwrap();

    info!("done");
}
