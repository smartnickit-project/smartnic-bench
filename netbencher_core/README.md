# NetBench-Core

A simple framework to start threads and collect their benchmark statistical data. 

## Quick start 

To check the framework works, simply use the following and ideally there would be no errors: 

```
cargo test
```

If the crate works fine, then you can use the following way to start collecting results from threads,
where each thread executes one null op per second:

```rust
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
```

Running such piece of code would generate the following results: 

```
Worker 1 started
Worker 0 started
Results: Throughput@0: 0.99 ops/s, Avg Latency: 0.00 ms, 99th Latency: 0.00 ms
Results: Throughput@0: 1.99 ops/s, Avg Latency: 0.00 ms, 99th Latency: 0.00 ms
Results: Throughput@0: 1.99 ops/s, Avg Latency: 0.00 ms, 99th Latency: 0.00 ms
Results: Throughput@0: 2.99 ops/s, Avg Latency: 0.00 ms, 99th Latency: 0.00 ms
```

---

For more example, please check the code snippets in the [examples](./examples/) folder. 
e.g., `cargo run --example  report_worker_stats`. 

For the coordinated reporters example, please note to first start the master then the reporter: 

```
# On the master machine:
cargo run --example coordinator_report_master --listen_addr="127.0.0.1:8888"

# On the reporter machine(s):
cargo run --example coordinator_report_worker --reporter_addr="127.0.0.1:8888"
```

Feel free to change the listen_addr or reporter_addr, as long as they are the same.