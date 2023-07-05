# Smartbench

Smart-bench is a rust-based benchmarking tool for [BlueField-series SmartNICs](https://www.nvidia.com/en-us/networking/products/data-processing-unit/). The purpose is to enable easy testing of BlueField-series SmartNICs. It is built on top of [DOCA](https://developer.nvidia.com/doca) and RDMA. The detailed results are summarized in our [paper](https://www.usenix.org/conference/osdi23/presentation/wei), please refer to here if you are interested. 

## Evaluated benchmarks

Smart-bench contains a set of benchmarks:

- [one_sided_rdma](benchs/one_sided_rdma/)
- [two_sided_rdma](benchs/two_sided_rdma/)
- [doca_dma](benchs/doca_dma/)

We pack our codes into a few building blocks:
- [bench_util](benchs/bench_util/): a set of utilities for benchmarks.
- [netbencher_core](netbencher_core/): a framework to start benchmark threads on different threads. 

We are continually maintaining the codebase to include features from future SmartNICs. 


## Quick start

Please refer to [README](benchs/docs/README.md).

## License Details

MIT License