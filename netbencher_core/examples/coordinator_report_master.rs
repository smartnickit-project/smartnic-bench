extern crate netbencher_core;

use std::time::Duration;

use clap::{Command, Arg, arg};
use simplelog::*;
use log::{info, LevelFilter};

use tokio::runtime::Runtime;

use netbencher_core::CoordinatedReporterMaster;

fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    ).unwrap();

    let matches = Command::new("bench example master")
        .arg(
            Arg::new("num_reports")
                .short('n')
                .long("num_reporters")
                .default_value("1"),
        )
        .arg(
            Arg::new("listen_addr")
                .short('r')
                .long("listen_addr")
                .required(true),
        )
        .arg(
            Arg::new("duration_secs")
                .short('d')
                .long("duration_secs")
                .default_value("20"),
        )
        .get_matches();

    Runtime::new().unwrap().block_on(async {
        let mut master = CoordinatedReporterMaster::new(
            *matches.get_one("num_reports")
                .expect("failed to get num reports"),
            matches.get_one::<String>("listen_addr").unwrap().to_string()
                .parse().unwrap(),
        )
        .await
        .expect("failed to create the master");

        master
            .report_event_loop(
                Duration::from_secs(*matches.get_one("duration_secs").unwrap()),
                Duration::from_secs(1),
            )
            .await
            .expect("Event loop report error");
    });

    info!("Master done");
}
