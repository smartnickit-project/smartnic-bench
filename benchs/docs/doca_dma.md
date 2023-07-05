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

In this section, we will talk about how to run a DMA test manually. To run our test, one server with DPU installed is required. You can refer to our paper for the hardware environment.

At SoC's terminal, type in:
```bash
./doca_rdma --addr ${server_ip}:${listen_port} -p ${pcie_dev}
```

- --addr: User must confirm availability of `server_ip` and `listen_port`, so that our benchmarks can build RDMA connections based on TCP. 

- -p: The `pcie_dev` is the PCIe dev id of DPU shown in the output of `lspci`. If you are using a 2-port DPU like us, just pick one device as the argument. Our experience shows that the number of devices used here will not affect performance.

This command will start a SoC instance, which will wait until the host instance is ready. Then, it will post DMA read/write requests to host. 

The default mode of bench is WRITE. If the SoC instance prints logs similar to the following, then you have succeeded:
```bash
06:26:59 [INFO] @0 Throughput: 3.971 Mops/s, Avg Latency: 0.25 µs
06:27:00 [INFO] @0 Throughput: 4.163 Mops/s, Avg Latency: 0.24 µs
06:27:01 [INFO] @0 Throughput: 4.164 Mops/s, Avg Latency: 0.24 µs
...
```



Then, at the host's terminal, type in:
```bash
./doca_rdma --server --addr ${server_ip}:${listen_port} -p ${pcie_dev}
```

- --addr User must make sure the `server_ip` and `listen_port` are consistent with SoC's. 

- -p The `pcie_dev` here represent the same PCIe device with that of SoC.

This command will start the host instance, which will accept connection from SoC and run silently. The host instance should run than SoC, and our default setting is as follows:

- SoC: `--life 15`, in seconds
- host: `--life 30`, in seconds

### Common arguments

The command line arguments of DOCA DMA are similar to RDMA tests, you can check them with `--help` or `-h`.

User shall provide arguments following these rules:

1. `threads * thread-gap <= random-space`
2. `payload <= random-space`
3. `payload <= thread-gap`
4. `batch-size * payload <= local-mr`

Otherwise, our program will try its best to rewrite the arguments to keep the invariants.

### Switch to READ mode

For DMA tests, we also use `--read` flag to specify READ test. So a read client shall use the command:

```bash
./doca_rdma --addr ${server_ip}:${listen_port} -p ${pcie_dev} --read
```

### Get the average latency

By default, our tests are targeted at maximizing throughput. 
We can switch to minimizing latency by using the following flags at the client:

```bash
./doca_rdma --addr ${server_ip}:${listen_port} -p ${pcie_dev} --threads 1 --batch-size 1
```

You can then specify the READ test with `--read`, and modify payload with `--payload`.

### Modify the batch size

We found that sending large batch of large DMA requests is very error-prone, we suggest you decrease the batch size as the payload grows:

```bash
./doca_rdma --addr ${server_ip}:${listen_port} -p ${pcie_dev} --batch-size n
```

### Modify the host region size

With larger payload, larger host memory region is required. User can modify the default host region to R bytes with the following:
```bash
./doca_rdma --server --addr ${server_ip}:${listen_port} -p ${pcie_dev} --random-space R
```

The host's `random_space` shall be equal to that of SoC.

### Fine-grained randomization

By default, each client thread will access the whole random region, but with different seed. We allow user to separate an area for each thread, and the thread will make sure its random access is limited in the area:

```bash
./doca_rdma --server --addr ${server_ip}:${listen_port} -p ${pcie_dev} --fixed
```

The area is of `thread_gap` size,  our bench makes sure that `thread_gap >= payload`, you can increase the thread_gap with `--thread-gap`, but you need to check that `threads * thread_gap <= random_space`.

## Results for reference

### DMA read

|Payload|Peek throughput (M reqs/sec)|
|---|---|
|16|10.5|
|64|10.5|
|256|10.5|
|1024|6.82|
|4096|3.87|
|16384|1.5|
|65536|0.39|

### DMA write

|Payload|Peek throughput (M reqs/sec)|
|---|---|
|16|10.19|
|64|10.19|
|256|10.19|
|1024|9.02|
|4096|3.71|
|16384|0.94|
|65536|0.23|