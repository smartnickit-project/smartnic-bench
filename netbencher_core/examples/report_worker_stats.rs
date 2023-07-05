#![feature(get_mut_unchecked)]

extern crate netbencher_core;

use std::sync::Arc;

use netbencher_core::{BenchRunner, SimpleBenchReporter};

fn main() {    
    let mut runner = BenchRunner::new(2);
    runner.run(
        // The evaluated function will increase the statics per second
        |worker_id, runner, mut stats, _| {
            println!("Worker {} started", worker_id);
            while runner.running() {
                std::thread::sleep(std::time::Duration::from_secs(1));
                unsafe { Arc::get_mut_unchecked(&mut stats).finished_one_op() };
            }
        },
        (),
    );

    let mut reporter = SimpleBenchReporter::new();
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let stat = runner.report(&mut reporter);
        println!("Results: {}", stat);

    }

    runner.stop().unwrap();
}
