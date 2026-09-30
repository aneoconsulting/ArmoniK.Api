# p1-ring: HEAD core against the patched core, from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Knobs and cores: runner.log. Per call: CPU (process) and wall, median [p10-p90] over the 10 chunks of each round's process; flt = minor faults, csw = voluntary + involuntary context switches (median of the processes); big = allocations of at least 1 MiB (a separate process with the allocation probe).

| workload | cell | core | CPU ms | wall ms | flt | csw | big | n |
|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | head | 8.220 [8.150-8.314] | 8.192 [8.105-8.469] | 0.1 | 221.0 | 16.00 | 30 |
| d/16MiB k=1 | A | patched | 8.264 [8.189-8.511] | 8.263 [8.145-9.437] | 0.1 | 224.6 | 16.00 | 30 |
| d/16MiB k=1 | D-retain | head | 8.262 [8.157-8.392] | 8.320 [8.213-9.067] | 0.1 | 205.7 | 0.00 | 30 |
| d/16MiB k=1 | D-retain | patched | 8.240 [8.121-8.413] | 8.332 [8.237-9.391] | 0.1 | 201.2 | 0.00 | 30 |
| d/16MiB k=1 | Cf-retain | head | 8.475 [8.159-8.829] | 8.401 [7.931-8.985] | 4.5 | 102.6 | 1.70 | 30 |
| d/16MiB k=1 | Cf-retain | patched | 8.451 [8.204-8.853] | 8.292 [7.759-10.746] | 0.0 | 106.4 | 0.00 | 30 |
| d/16MiB k=1 | Cf-q-retain | head | 8.511 [8.345-9.030] | 8.353 [7.560-10.136] | 0.1 | 160.6 | 1.80 | 30 |
| d/16MiB k=1 | Cf-q-retain | patched | 9.065 [8.676-9.267] | 8.327 [7.792-9.622] | 0.1 | 194.3 | 0.00 | 30 |
| d/16MiB k=8 | A | head | 9.653 [9.533-9.845] | 7.702 [7.404-7.855] | 1.7 | 203.9 | 16.00 | 30 |
| d/16MiB k=8 | A | patched | 9.630 [9.471-9.839] | 7.520 [7.341-7.840] | 1.7 | 194.0 | 16.00 | 30 |
| d/16MiB k=8 | D-retain | head | 9.603 [9.431-9.750] | 7.536 [7.395-7.749] | 0.2 | 196.7 | 0.00 | 30 |
| d/16MiB k=8 | D-retain | patched | 9.649 [9.492-9.725] | 7.541 [7.385-7.810] | 0.2 | 199.4 | 0.00 | 30 |
| d/16MiB k=8 | Cf-retain | head | 9.488 [9.356-9.621] | 8.440 [8.165-8.709] | 4.3 | 74.3 | 1.98 | 30 |
| d/16MiB k=8 | Cf-retain | patched | 9.512 [9.351-9.748] | 8.382 [8.201-8.699] | 0.3 | 72.2 | 0.00 | 30 |
| d/16MiB k=8 | Cf-q-retain | head | 9.513 [8.868-10.903] | 8.293 [8.043-8.717] | 458.4 | 128.4 | 4.08 | 30 |
| d/16MiB k=8 | Cf-q-retain | patched | 9.333 [8.810-10.456] | 8.435 [8.163-8.644] | 213.0 | 131.7 | 3.60 | 30 |
| d/4MiB k=1 | A | head | 2.112 [2.094-2.162] | 2.238 [2.202-2.295] | 0.0 | 61.3 | 4.00 | 30 |
| d/4MiB k=1 | A | patched | 2.100 [2.076-2.124] | 2.218 [2.182-2.265] | 0.0 | 60.8 | 4.00 | 30 |
| d/4MiB k=1 | D-retain | head | 2.173 [2.153-2.189] | 2.338 [2.304-2.375] | 0.0 | 57.3 | 0.00 | 30 |
| d/4MiB k=1 | D-retain | patched | 2.172 [2.153-2.195] | 2.320 [2.285-2.377] | 0.0 | 57.9 | 0.00 | 30 |
| d/4MiB k=1 | Cf-retain | head | 2.060 [2.039-2.094] | 2.321 [2.263-2.524] | 0.0 | 27.3 | 0.00 | 30 |
| d/4MiB k=1 | Cf-retain | patched | 2.084 [2.052-2.204] | 2.404 [2.315-2.513] | 0.0 | 30.0 | 0.00 | 30 |
| d/4MiB k=1 | Cf-q-retain | head | 2.089 [2.028-2.226] | 2.383 [2.346-2.493] | 0.0 | 39.5 | 0.00 | 30 |
| d/4MiB k=1 | Cf-q-retain | patched | 2.114 [2.029-2.254] | 2.340 [2.267-2.427] | 0.0 | 43.0 | 0.00 | 30 |
| c/P5.4 k=1 | A | head | 2.613 [2.394-2.721] | 6.366 [4.939-6.996] | 0.0 | 100.9 | 4.00 | 30 |
| c/P5.4 k=1 | A | patched | 2.512 [2.188-2.726] | 5.872 [3.323-7.057] | 0.0 | 86.4 | 4.00 | 30 |
| c/P5.4 k=1 | D-retain | head | 2.560 [2.218-2.740] | 5.858 [3.527-6.964] | 0.0 | 100.3 | 0.00 | 30 |
| c/P5.4 k=1 | D-retain | patched | 2.626 [2.366-2.740] | 6.265 [4.023-7.025] | 0.0 | 102.4 | 0.00 | 30 |
| c/P5.4 k=1 | Cf-retain | head | 2.011 [1.979-2.058] | 3.285 [2.985-3.624] | 0.0 | 19.1 | 0.00 | 30 |
| c/P5.4 k=1 | Cf-retain | patched | 2.019 [1.987-2.071] | 3.750 [3.206-4.607] | 0.0 | 21.0 | 0.00 | 30 |
| c/P5.4 k=1 | Cf-q-retain | head | 1.947 [1.917-1.979] | 3.368 [2.912-4.263] | 0.1 | 14.5 | 0.00 | 30 |
| c/P5.4 k=1 | Cf-q-retain | patched | 1.943 [1.911-1.981] | 3.456 [2.898-4.468] | 0.0 | 15.0 | 0.00 | 30 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | core | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | head | 2.059 | 0.017 | 6.397 |  |
| d/16MiB k=1 | Cf-retain | patched | 2.112 | 0.017 | 6.311 |  |
| d/16MiB k=1 | Cf-q-retain | head | 0.000 | 2.088 | 6.514 |  |
| d/16MiB k=1 | Cf-q-retain | patched | 0.000 | 2.182 | 6.842 |  |
| d/16MiB k=8 | Cf-retain | head | 2.752 | 0.008 | 6.753 |  |
| d/16MiB k=8 | Cf-retain | patched | 2.721 | 0.007 | 6.805 |  |
| d/16MiB k=8 | Cf-q-retain | head | 0.000 | 2.459 | 7.368 |  |
| d/16MiB k=8 | Cf-q-retain | patched | 0.000 | 2.086 | 7.355 |  |
| d/4MiB k=1 | Cf-retain | head | 0.514 | 0.013 | 1.540 |  |
| d/4MiB k=1 | Cf-retain | patched | 0.511 | 0.015 | 1.559 |  |
| d/4MiB k=1 | Cf-q-retain | head | 0.000 | 0.515 | 1.580 |  |
| d/4MiB k=1 | Cf-q-retain | patched | 0.000 | 0.531 | 1.620 |  |
| c/P5.4 k=1 | Cf-retain | head | 0.543 | 0.017 | 1.459 |  |
| c/P5.4 k=1 | Cf-retain | patched | 0.556 | 0.017 | 1.452 |  |
| c/P5.4 k=1 | Cf-q-retain | head | 0.000 | 0.508 | 1.432 |  |
| c/P5.4 k=1 | Cf-q-retain | patched | 0.000 | 0.506 | 1.430 |  |

Median per-call CPU per process (ms), rounds in run order:

- d/16MiB k=1 A: head 8.246 8.246 8.198; patched 8.362 8.253 8.254
- d/16MiB k=1 D-retain: head 8.363 8.169 8.273; patched 8.393 8.240 8.135
- d/16MiB k=1 Cf-retain: head 8.757 8.462 8.184; patched 8.711 8.346 8.379
- d/16MiB k=1 Cf-q-retain: head 8.507 8.880 8.377; patched 9.126 8.934 9.109
- d/16MiB k=8 A: head 9.591 9.723 9.691; patched 9.717 9.503 9.690
- d/16MiB k=8 D-retain: head 9.645 9.568 9.585; patched 9.678 9.626 9.588
- d/16MiB k=8 Cf-retain: head 9.491 9.483 9.495; patched 9.491 9.603 9.487
- d/16MiB k=8 Cf-q-retain: head 9.481 9.590 9.543; patched 9.302 9.660 9.108
- d/4MiB k=1 A: head 2.136 2.105 2.106; patched 2.116 2.082 2.100
- d/4MiB k=1 D-retain: head 2.173 2.171 2.174; patched 2.174 2.166 2.173
- d/4MiB k=1 Cf-retain: head 2.065 2.059 2.053; patched 2.134 2.071 2.073
- d/4MiB k=1 Cf-q-retain: head 2.090 2.039 2.099; patched 2.236 2.081 2.228
- c/P5.4 k=1 A: head 2.619 2.553 2.681; patched 2.231 2.708 2.524
- c/P5.4 k=1 D-retain: head 2.228 2.588 2.670; patched 2.522 2.653 2.658
- c/P5.4 k=1 Cf-retain: head 2.027 2.027 1.984; patched 2.011 2.011 2.041
- c/P5.4 k=1 Cf-q-retain: head 1.944 1.928 1.961; patched 1.934 1.975 1.926
