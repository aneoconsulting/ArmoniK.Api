# spares24: HEAD core against the patched core, from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Knobs and cores: runner.log. Per call: CPU (process) and wall, median [p10-p90] over the 10 chunks of each round's process; flt = minor faults, csw = voluntary + involuntary context switches (median of the processes); big = allocations of at least 1 MiB (a separate process with the allocation probe).

| workload | cell | core | CPU ms | wall ms | flt | csw | big | n |
|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | head | 8.303 [8.083-8.632] | 8.403 [7.552-8.996] | 4.5 | 94.1 | 1.70 | 30 |
| d/16MiB k=1 | Cf-retain | patched | 8.524 [8.298-8.683] | 8.219 [7.445-8.834] | 0.1 | 108.0 | 0.03 | 30 |
| d/16MiB k=1 | Cf-q-retain | head | 9.033 [8.498-9.288] | 8.170 [7.420-9.206] | 0.1 | 207.2 | 1.60 | 30 |
| d/16MiB k=1 | Cf-q-retain | patched | 8.590 [8.391-9.116] | 8.211 [7.356-9.809] | 0.1 | 174.0 | 0.00 | 30 |
| d/16MiB k=8 | Cf-retain | head | 9.490 [9.344-9.679] | 8.301 [8.063-8.657] | 17.1 | 78.5 | 1.98 | 30 |
| d/16MiB k=8 | Cf-retain | patched | 9.659 [9.491-9.831] | 8.318 [8.166-8.693] | 0.3 | 81.7 | 0.00 | 30 |
| d/16MiB k=8 | Cf-q-retain | head | 9.914 [8.820-11.283] | 8.401 [8.021-8.768] | 564.2 | 119.9 | 4.03 | 30 |
| d/16MiB k=8 | Cf-q-retain | patched | 8.757 [8.422-9.338] | 8.213 [7.990-8.424] | 3.4 | 125.9 | 1.40 | 30 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | core | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | head | 2.083 | 0.017 | 6.226 |  |
| d/16MiB k=1 | Cf-retain | patched | 2.074 | 0.019 | 6.397 |  |
| d/16MiB k=1 | Cf-q-retain | head | 0.000 | 2.154 | 6.918 |  |
| d/16MiB k=1 | Cf-q-retain | patched | 0.000 | 2.098 | 6.544 |  |
| d/16MiB k=8 | Cf-retain | head | 2.771 | 0.007 | 6.789 |  |
| d/16MiB k=8 | Cf-retain | patched | 2.737 | 0.007 | 6.916 |  |
| d/16MiB k=8 | Cf-q-retain | head | 0.000 | 2.587 | 7.227 |  |
| d/16MiB k=8 | Cf-q-retain | patched | 0.000 | 1.674 | 7.182 |  |

Median per-call CPU per process (ms), rounds in run order:

- d/16MiB k=1 A: head ; patched 
- d/16MiB k=1 D-retain: head ; patched 
- d/16MiB k=1 Cf-retain: head 8.249 8.439 8.287; patched 8.361 8.544 8.519
- d/16MiB k=1 Cf-q-retain: head 9.115 8.525 9.046; patched 8.793 8.572 8.600
- d/16MiB k=8 A: head ; patched 
- d/16MiB k=8 D-retain: head ; patched 
- d/16MiB k=8 Cf-retain: head 9.457 9.579 9.535; patched 9.659 9.692 9.628
- d/16MiB k=8 Cf-q-retain: head 9.387 10.792 9.784; patched 8.768 8.579 8.839
- d/4MiB k=1 A: head ; patched 
- d/4MiB k=1 D-retain: head ; patched 
- d/4MiB k=1 Cf-retain: head ; patched 
- d/4MiB k=1 Cf-q-retain: head ; patched 
- c/P5.4 k=1 A: head ; patched 
- c/P5.4 k=1 D-retain: head ; patched 
- c/P5.4 k=1 Cf-retain: head ; patched 
- c/P5.4 k=1 Cf-q-retain: head ; patched 
