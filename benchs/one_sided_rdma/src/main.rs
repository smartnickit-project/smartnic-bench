#![feature(get_mut_unchecked)]

mod bootstrap;
use bootstrap::*;

use bench_util::args::*;
use clap::Parser;

use log::*;
use simplelog::*;

fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    ).unwrap();
    let mut args = CmdlineArgs::parse();
    args.coordinate();
    // main_inner will create threads and wait for them to exit 
    main_inner(args);
}

fn main_inner(args: CmdlineArgs) {
    
    debug!(
        "Sanity check parameters: payload {}, nthreads {}, use READ {}, signaled: {}, doorbell: {}",
        args.payload,
        args.threads,
        args.read,
        args.signaled,
        args.doorbell,
    );
    
    if args.server {
        bootstrap_server(args);
    } else {
        bootstrap_client(args);
    }
}