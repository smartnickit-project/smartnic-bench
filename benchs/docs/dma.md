# DOCA DMA

## Quick start

### Build

DOCA DMA required a DOCA environment, you can refer to our [rust-doca](https://ipads.se.sjtu.edu.cn:1312/distributed-rdma-serverless/smartnic-project/rust-doca) or [DOCA SDK](https://docs.nvidia.com/doca/sdk/index.html) for DOCA installation.

To build all binary files required for our benchmark, run:
```bash
cd bench/doca_dma
cargo build --release
```

If your OFED version is >= 5.0, we offer a cargo feature to enable your building:
```bash
cargo build --release --features "OFED_5_4"
```

If to build on DPU SoC, another feature is required:
```bash
cargo build --release --features "OFED_5_4 ARM"
```

Having the similar output on the terminal means you have succeeded:
```bash
Finished release [optimized] target(s) in 4.09s
```

All binary files can be found in `bench/target/release`.

### Run

In this section, we will talk about how to run a RDMA test manually. To run our test, one server and multiple clients are required. All of them must be installed with Infiniband RDMA NICs. You can refer to our paper for the hardware environment.

At the server's terminal, type in:
```
./one_sided_rdma --server --addr ${server_ip}:${listen_port} 
```

User must confirm availability of `server_ip` and `listen_port`, so that our benchmarks can build RDMA connections based on TCP.

At each client's terminal, type in:
```
./one_sided_rdma --addr ${server_ip}:${listen_port}
```
The `server_ip` and `listen_port` is just the same as server's.

The default mode of bench is WRITE. If clients print logs similar to the following, then you have succeeded:
```
06:54:10 [INFO] @0 Throughput: 7.72 Mops/s, Avg Latency: 0.13 µs
06:54:11 [INFO] @0 Throughput: 7.95 Mops/s, Avg Latency: 0.13 µs
06:54:12 [INFO] @0 Throughput: 7.96 Mops/s, Avg Latency: 0.13 µs
...
06:54:10 [INFO] @0 Throughput: 7.72 Mops/s, Avg Latency: 0.13 µs
06:54:11 [INFO] @0 Throughput: 7.95 Mops/s, Avg Latency: 0.13 µs
06:54:12 [INFO] @0 Throughput: 7.96 Mops/s, Avg Latency: 0.13 µs
...
```

### Common arguments

Our benchmark support configuring tests with command line arguments. Here are some common ones:

|Client side flag|Description|Default|
|---|---|---|
|--payload|Payload(byte) for RDMA requests.|32|
|--client-id|ID of the benchmarking client. Clients will generate different access patterns according to their client ids.|0|
|--threads|Threads number.|1|
|--life|How long will the client live(seconds).|15|
|--huge-page|Whether or not to use huge page for memory region.|N/A|

|Server side flag|Description|Default|
|---|---|---|
|--huge-page|Whether or not to use huge page for memory region.|N/A|
|--life|How long will the server live(seconds).|30|

You can check more flags with `--help`.

### Switch to READ mode

For one-sided tests, we use `--read` flag to specify READ test. So a read client shall use the command:

```bash
./one_sided_rdma --addr ${server_ip}:${listen_port} --read
```

### Get the average latency

By default, our tests are targeted at maximizing throughput. 
We can switch to minimizing latency by using the following flags at the client:

```bash
./one_sided_rdma --addr ${server_ip}:${listen_port} --threads 1 --factor 1 --latency
```

You can then specify the READ test with `--read`, and modify payload with `--payload`.

### Change NIC device

By default, our tests use the first NIC device found. Sometimes, the RNIC you want to test might not be not the first. In these cases, we offer `--nic-idx` to allow user to choose NIC device.

You can check your NIC device id using:

```bash
ibv_devinfo
```

Client can use NIC n by typing:

```bash
./one_sided_rdma --addr ${server_ip}:${listen_port} --nic-idx n
```

Similarly, server can use the following:

```bash
./one_sided_rdma --server --addr ${server_ip}:${listen_port} --nic-idx n
```

### Accelerate posting w/ doorbell

In one-sided tests, client can apply doorbell batching to accelerating posting, but according to our experience, we suggest you use it carefully in some cases. You can refer to our paper for details.

The doorbell batching is activated by typing:

```bash
./one_sided_rdma --addr ${server_ip}:${listen_port} --doorbell
```

If you want to change the batch size (i.e. factor) or the doorbell size (i.e. db_size), remember to make sure that `db_size <= factor`.

### Add reporting for clients

Collecting throughput or latency logs from multiple clients is troublesome. We provide a optional report function, which will collect each client's average throughput and latency and merge them.

To use this function, first choose a unused port at server, denoted as `report_port`, this port must be different from `listen_port`. Add this argument to server with `--report-addr` flag:

```bash
./one_sided_rdma --server --addr ${server_ip}:${listen_port} --report-addr ${server_ip}:${report_port}
```

And for each client, use:

```bash
./one_sided_rdma --addr ${server_ip}:${listen_port} --report-addr ${server_ip}:${report_port}
```