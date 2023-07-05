# TWO SIDED RDMA
# TWO SIDED RDMA

## Quick start

### Build

Read [one_sided_rdma](one_sided_rdma.md) for reference.
Read [one_sided_rdma](one_sided_rdma.md) for reference.

Our two_sided bench is in `bench/two_sided_rdma`

Our two_sided bench is in `bench/two_sided_rdma`

### Run

In this section, we will talk about how to run a SEND/RECV test manually. To run our test, one server and multiple clients are required. All of them must be installed with Infiniband RDMA NICs. You can refer to our paper for the hardware environment.
In this section, we will talk about how to run a SEND/RECV test manually. To run our test, one server and multiple clients are required. All of them must be installed with Infiniband RDMA NICs. You can refer to our paper for the hardware environment.

At the server's terminal, type in:
```bash
./two_sided_rdma --server --addr ${server_ip}:${listen_port} 
```

User must confirm availability of `server_ip` and `listen_port`, so that our benchmarks can build RDMA connections based on TCP.

At each client's terminal, type in:
```bash
./two_sided_rdma --addr ${server_ip}:${listen_port}
```
The `server_ip` and `listen_port` is just the same as server's.

If clients print logs similar to the following, then you have succeeded:

```bash
08:51:59 [INFO] @0 Throughput: 5.20 Mops/s, Avg Latency: 0.19 µs
08:52:00 [INFO] @0 Throughput: 5.22 Mops/s, Avg Latency: 0.19 µs
08:52:01 [INFO] @0 Throughput: 5.21 Mops/s, Avg Latency: 0.19 µs
...
```

### Common arguments

Our benchmark support configuring tests with command line arguments. Here are some common ones:

|Client side flag|Description|Default|
|---|---|---|
|--payload|Payload(byte) for RDMA requests.|32|
|--client-id|ID of the benchmarking client. Clients will generate different access patterns according to their client ids.|0|
|--client-id|ID of the benchmarking client. Clients will generate different access patterns according to their client ids.|0|
|--threads|Threads number.|1|
|--life|How long will the client live(seconds).|15|
|--huge-page|Whether or not to use huge page for memory region.|N/A|
|--huge-page|Whether or not to use huge page for memory region.|N/A|

|Server side flag|Description|Default|
|---|---|---|
|--huge-page|Whether or not to use huge page for memory region.|N/A|
|--life|How long will the server live(seconds).|30|

You can check more flags with `--help`.

User shall provide arguments following these rules:

1. `threads * thread-gap <= random-space`
2. `payload <= random-space`
3. `payload <= thread-gap`
4. `factor * payload <= local-mr`

Otherwise, our program will try its best to rewrite the arguments to keep the invariants.

### Get the average latency

By default, our tests are targeted at maximizing throughput. 
We can switch to minimizing latency by using the following flags at the client:

```bash
./two_sided_rdma --addr ${server_ip}:${listen_port} --threads 1 --factor 1 --latency-test
```

And the server side:
And the server side:

```bash
./two_sided_rdma --server --addr ${server_ip}:${listen_port} --threads 1 --latency-test
```

### Change NIC device

By default, our tests use the first NIC device found. Sometimes, the RNIC you want to test might not be not the first. In these cases, we offer `--nic-idx` to allow user to choose NIC device.

You can check your NIC device id using

```bash
ibv_devinfo
```

Client can use NIC n by typing:

```bash
./two_sided_rdma --addr ${server_ip}:${listen_port} --nic-idx n
```

Similarly, server can use the following:

```bash
./two_sided_rdma --server --addr ${server_ip}:${listen_port} --nic-idx n
```

In our `two_sided_client` and `two_sided_server`, qps can be distributed evenly across multiple network ports, you can specify the number of network ports to use with `--nic-num n`.

For example: 

```bash
# client-side
./two_sided_rdma --addr ${server_ip}:${listen_port} --nic-num 2
```

```bash
# server-side
./two_sided_rdma --server --addr ${server_ip}:${listen_port} --nic-num 2
```

### Accelerate posting w/ doorbell

In two-sided tests, client and server can both apply doorbell batching to accelerating posting, but according to our experience, we suggest you use it carefully in some cases. You can refer to our paper for details.

The doorbell batching is activated by typing:

```bash
./two_sided_rdma --addr ${server_ip}:${listen_port} --doorbell
```

If you want to change the batch size (i.e. factor) or the doorbell size (i.e. db_size), remember to make sure that `db_size <= factor`.

### Add reporting for clients

Collecting throughput or latency logs from different clients is troublesome. We provide a optional report function, which will collect each client's average throughput and latency and merge them.

To use this function, first choose a unused port at server, denoted as `report_port`, this port must be different from `listen_port`. Add this argument to server with `--report-addr` flag:

```bash
./two_sided_rdma --server --addr ${server_ip}:${listen_port} --report --report-addr ${server_ip}:${report_port}
```

And for each client, use:

```bash
./two_sided_rdma --addr ${server_ip}:${listen_port} --report --report-addr ${server_ip}:${report_port}
```

P.S. This flag is not recommended when DPU SoC is used as a server.