# Latency and throughput results of SmartNIC

## Crate descriptions

|Application|Description|
|-----|-------------------|
|`one_sided_rdma`|The crate for one-sided RDMA microbenchmarks.|
|`two_sided_rdma`|The crate for two-sided UD RDMA microbenchmarks.|
|`doca_dma`      |The crate for doca DMA microbenchmarks.|

|Library|Description|
|-----|-------------------|
|`bench_util`|A library crate containing common functions (e.g. doorbell batching, rdtsc-counter, command-line arguments) for the bench.|

## Run Evaluations

For how to run each of the benchmark, please refer to the following docs: 

- [ONE SIDED RDMA](one_sided_rdma.md)
- [TWO SIDED RDMA](two_sided_rdma.md)
- [DOCA DMA](doca_dma.md)
