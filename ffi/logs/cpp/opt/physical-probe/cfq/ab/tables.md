# ab: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median [p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary context switches per call (median of the processes).

## measure: main figures (every process under AB_ENV, the allocator setting; 3 rounds)

| workload | cell | arm | CPU ms | wall ms | flt | csw | n |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 8.496 [8.198-8.914] | 8.147 [7.328-9.392] | 4.5 | 123.1 | 30 |
| d/16MiB k=1 | Cf-retain | curslot | 8.316 [8.195-8.543] | 8.277 [7.548-9.075] | 4.3 | 97.9 | 30 |
| d/16MiB k=1 | Cf-retain | q1 | 8.260 [8.068-8.520] | 8.365 [7.784-9.668] | 4.5 | 88.9 | 30 |
| d/16MiB k=1 | Cf-retain | q1slot | 8.313 [8.148-8.507] | 7.471 [7.210-9.566] | 0.2 | 107.6 | 30 |
| d/16MiB k=1 | Cf-q-retain | cur | 9.151 [8.728-9.462] | 8.460 [7.642-9.546] | 0.1 | 223.2 | 30 |
| d/16MiB k=1 | Cf-q-retain | curslot | 9.437 [8.891-9.620] | 8.381 [7.741-8.842] | 0.1 | 249.7 | 30 |
| d/16MiB k=1 | Cf-q-retain | q1 | 8.825 [8.672-9.397] | 8.337 [7.684-9.721] | 0.1 | 180.9 | 30 |
| d/16MiB k=1 | Cf-q-retain | q1slot | 9.133 [8.865-9.637] | 8.330 [7.630-9.181] | 0.1 | 218.3 | 30 |
| d/16MiB k=8 | Cf-retain | cur | 9.527 [9.356-9.740] | 8.344 [8.133-8.659] | 20.1 | 71.8 | 30 |
| d/16MiB k=8 | Cf-retain | curslot | 9.502 [9.318-9.642] | 8.255 [8.070-8.446] | 16.9 | 73.3 | 30 |
| d/16MiB k=8 | Cf-retain | q1 | 9.481 [9.387-9.664] | 8.268 [8.071-8.527] | 13.8 | 73.6 | 30 |
| d/16MiB k=8 | Cf-retain | q1slot | 9.546 [9.347-9.684] | 8.272 [7.988-8.526] | 17.1 | 72.8 | 30 |
| d/16MiB k=8 | Cf-q-retain | cur | 8.600 [8.223-9.147] | 8.219 [7.939-8.469] | 3.4 | 117.5 | 30 |
| d/16MiB k=8 | Cf-q-retain | curslot | 8.403 [8.252-8.963] | 8.150 [7.928-8.419] | 0.1 | 105.2 | 30 |
| d/16MiB k=8 | Cf-q-retain | q1 | 8.517 [8.173-9.254] | 8.243 [7.932-8.470] | 3.4 | 113.4 | 30 |
| d/16MiB k=8 | Cf-q-retain | q1slot | 8.287 [8.199-8.516] | 8.135 [7.910-8.472] | 0.1 | 91.7 | 30 |
| d/4MiB k=1 | Cf-retain | cur | 2.054 [2.024-2.092] | 2.435 [2.315-2.490] | 0.0 | 30.8 | 30 |
| d/4MiB k=1 | Cf-retain | curslot | 2.053 [2.020-2.092] | 2.440 [2.293-2.537] | 0.0 | 31.3 | 30 |
| d/4MiB k=1 | Cf-retain | q1 | 2.046 [2.016-2.075] | 2.378 [2.283-2.476] | 0.0 | 33.4 | 30 |
| d/4MiB k=1 | Cf-retain | q1slot | 2.058 [2.027-2.084] | 2.447 [2.254-2.508] | 0.0 | 33.0 | 30 |
| d/4MiB k=1 | Cf-q-retain | cur | 2.096 [2.045-2.174] | 2.309 [2.219-2.429] | 0.0 | 41.0 | 30 |
| d/4MiB k=1 | Cf-q-retain | curslot | 2.263 [2.125-2.335] | 2.425 [2.279-2.468] | 0.0 | 58.8 | 30 |
| d/4MiB k=1 | Cf-q-retain | q1 | 2.065 [2.037-2.129] | 2.399 [2.320-2.458] | 0.0 | 38.7 | 30 |
| d/4MiB k=1 | Cf-q-retain | q1slot | 2.112 [2.083-2.165] | 2.408 [2.283-2.496] | 0.1 | 49.0 | 30 |
| c/P5.4 k=1 | Cf-retain | cur | 2.057 [1.987-2.094] | 3.839 [2.919-4.586] | 0.0 | 20.4 | 30 |
| c/P5.4 k=1 | Cf-retain | curslot | 2.024 [1.991-2.087] | 3.649 [2.899-4.235] | 0.0 | 19.4 | 30 |
| c/P5.4 k=1 | Cf-retain | q1 | 2.058 [2.001-2.132] | 4.285 [3.073-5.107] | 0.0 | 20.6 | 30 |
| c/P5.4 k=1 | Cf-retain | q1slot | 2.063 [1.991-2.096] | 4.298 [3.269-4.667] | 0.0 | 20.3 | 30 |
| c/P5.4 k=1 | Cf-q-retain | cur | 2.008 [1.963-2.065] | 3.540 [2.977-4.543] | 0.0 | 15.5 | 30 |
| c/P5.4 k=1 | Cf-q-retain | curslot | 2.040 [2.001-2.074] | 4.189 [3.168-4.846] | 0.0 | 14.9 | 30 |
| c/P5.4 k=1 | Cf-q-retain | q1 | 2.038 [1.986-2.066] | 4.253 [3.273-4.738] | 0.0 | 14.9 | 30 |
| c/P5.4 k=1 | Cf-q-retain | q1slot | 2.027 [2.006-2.059] | 3.656 [3.111-4.268] | 0.0 | 14.9 | 30 |

Run-queue wait per call (ms; schedstat's second field, every thread of the class; median of the processes) and the batch trace (blocking path: per batch of k calls, median over the batches of the processes, ms): batch wall, one call's duration median [p10-p90], first call start to last call end (span), batch start to its last call's start (dispatch), last call end to batch end (completion).

| workload | cell | arm | wait: caller | main | tokio-rt-worker | event_engine | batch wall | call [p10-p90] | span | dispatch | completion |
|---|---|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 0.000 | 0.000 | 0.001 |  | 7.960 | 7.897 [7.146-8.763] | 7.897 | 0.008 | 0.052 |
| d/16MiB k=1 | Cf-retain | curslot | 0.000 | 0.000 | 0.001 |  | 8.021 | 7.966 [7.232-8.907] | 7.966 | 0.008 | 0.052 |
| d/16MiB k=1 | Cf-retain | q1 | 0.000 | 0.000 | 0.001 |  | 8.207 | 8.147 [7.249-8.913] | 8.147 | 0.008 | 0.051 |
| d/16MiB k=1 | Cf-retain | q1slot | 0.000 | 0.000 | 0.002 |  | 7.502 | 7.440 [6.968-8.696] | 7.440 | 0.008 | 0.051 |
| d/16MiB k=1 | Cf-q-retain | cur | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=1 | Cf-q-retain | curslot | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=1 | Cf-q-retain | q1slot | 0.000 | 0.000 | 0.002 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-retain | cur | 0.277 | 0.000 | 0.217 |  | 66.927 | 64.232 [56.983-67.911] | 66.867 | 0.868 | 0.016 |
| d/16MiB k=8 | Cf-retain | curslot | 0.234 | 0.000 | 0.217 |  | 66.420 | 63.202 [55.091-66.497] | 66.355 | 0.870 | 0.050 |
| d/16MiB k=8 | Cf-retain | q1 | 0.281 | 0.000 | 0.217 |  | 66.421 | 62.999 [55.089-67.328] | 66.360 | 0.870 | 0.015 |
| d/16MiB k=8 | Cf-retain | q1slot | 0.220 | 0.000 | 0.207 |  | 66.903 | 62.411 [55.122-66.840] | 66.877 | 0.858 | 0.019 |
| d/16MiB k=8 | Cf-q-retain | cur | 0.000 | 0.000 | 0.002 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-q-retain | curslot | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.002 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-q-retain | q1slot | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/4MiB k=1 | Cf-retain | cur | 0.000 | 0.000 | 0.000 |  | 2.398 | 2.347 [2.133-2.553] | 2.347 | 0.007 | 0.013 |
| d/4MiB k=1 | Cf-retain | curslot | 0.000 | 0.000 | 0.000 |  | 2.432 | 2.406 [2.143-2.582] | 2.406 | 0.007 | 0.013 |
| d/4MiB k=1 | Cf-retain | q1 | 0.000 | 0.000 | 0.000 |  | 2.353 | 2.324 [2.109-2.517] | 2.324 | 0.007 | 0.013 |
| d/4MiB k=1 | Cf-retain | q1slot | 0.000 | 0.000 | 0.000 |  | 2.365 | 2.348 [2.116-2.566] | 2.348 | 0.007 | 0.012 |
| d/4MiB k=1 | Cf-q-retain | cur | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| d/4MiB k=1 | Cf-q-retain | curslot | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| d/4MiB k=1 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| d/4MiB k=1 | Cf-q-retain | q1slot | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| c/P5.4 k=1 | Cf-retain | cur | 0.000 | 0.000 | 0.000 |  | 3.219 | 3.160 [2.770-6.007] | 3.160 | 0.008 | 0.054 |
| c/P5.4 k=1 | Cf-retain | curslot | 0.000 | 0.000 | 0.000 |  | 2.899 | 2.837 [2.734-5.447] | 2.837 | 0.008 | 0.052 |
| c/P5.4 k=1 | Cf-retain | q1 | 0.000 | 0.000 | 0.000 |  | 3.404 | 3.341 [2.757-6.841] | 3.341 | 0.008 | 0.051 |
| c/P5.4 k=1 | Cf-retain | q1slot | 0.000 | 0.000 | 0.000 |  | 3.321 | 3.260 [2.782-6.700] | 3.260 | 0.008 | 0.053 |
| c/P5.4 k=1 | Cf-q-retain | cur | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| c/P5.4 k=1 | Cf-q-retain | curslot | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| c/P5.4 k=1 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| c/P5.4 k=1 | Cf-q-retain | q1slot | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |

Context switches per call by thread class, voluntary / involuntary (median of the processes), and the server during the loop (AK_SERVER_PID: CPU per call, ms; its threads' voluntary switches are not read):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine | server CPU |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 6.0 / 0.0 | 1.0 / 0.0 | 115.8 / 0.0 |  | 10.807 |
| d/16MiB k=1 | Cf-retain | curslot | 6.0 / 0.0 | 1.0 / 0.0 | 90.9 / 0.0 |  | 10.846 |
| d/16MiB k=1 | Cf-retain | q1 | 6.0 / 0.0 | 1.0 / 0.0 | 81.8 / 0.0 |  | 11.211 |
| d/16MiB k=1 | Cf-retain | q1slot | 6.0 / 0.0 | 1.0 / 0.0 | 100.6 / 0.0 |  | 10.749 |
| d/16MiB k=1 | Cf-q-retain | cur | 0.0 / 0.0 | 9.0 / 0.0 | 214.2 / 0.0 |  | 10.955 |
| d/16MiB k=1 | Cf-q-retain | curslot | 0.0 / 0.0 | 8.9 / 0.0 | 240.8 / 0.0 |  | 11.072 |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.0 / 0.0 | 5.0 / 0.0 | 175.9 / 0.0 |  | 11.075 |
| d/16MiB k=1 | Cf-q-retain | q1slot | 0.0 / 0.0 | 5.2 / 0.0 | 213.1 / 0.0 |  | 10.942 |
| d/16MiB k=8 | Cf-retain | cur | 8.6 / 0.1 | 0.1 / 0.0 | 63.0 / 0.0 |  | 20.110 |
| d/16MiB k=8 | Cf-retain | curslot | 8.6 / 0.1 | 0.1 / 0.0 | 64.4 / 0.1 |  | 19.551 |
| d/16MiB k=8 | Cf-retain | q1 | 8.6 / 0.0 | 0.1 / 0.0 | 64.7 / 0.1 |  | 19.726 |
| d/16MiB k=8 | Cf-retain | q1slot | 8.6 / 0.1 | 0.1 / 0.0 | 63.9 / 0.0 |  | 19.724 |
| d/16MiB k=8 | Cf-q-retain | cur | 0.0 / 0.0 | 4.0 / 0.0 | 113.2 / 0.0 |  | 19.436 |
| d/16MiB k=8 | Cf-q-retain | curslot | 0.0 / 0.0 | 4.1 / 0.0 | 101.1 / 0.0 |  | 19.520 |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.0 / 0.0 | 4.1 / 0.0 | 109.2 / 0.0 |  | 19.355 |
| d/16MiB k=8 | Cf-q-retain | q1slot | 0.0 / 0.0 | 3.9 / 0.0 | 87.8 / 0.0 |  | 19.485 |
| d/4MiB k=1 | Cf-retain | cur | 2.0 / 0.0 | 1.0 / 0.0 | 27.8 / 0.0 |  | 2.397 |
| d/4MiB k=1 | Cf-retain | curslot | 2.0 / 0.0 | 1.0 / 0.0 | 28.3 / 0.0 |  | 2.512 |
| d/4MiB k=1 | Cf-retain | q1 | 2.0 / 0.0 | 1.0 / 0.0 | 30.4 / 0.0 |  | 2.448 |
| d/4MiB k=1 | Cf-retain | q1slot | 2.0 / 0.0 | 1.0 / 0.0 | 30.0 / 0.0 |  | 2.466 |
| d/4MiB k=1 | Cf-q-retain | cur | 0.0 / 0.0 | 3.0 / 0.0 | 38.0 / 0.0 |  | 2.392 |
| d/4MiB k=1 | Cf-q-retain | curslot | 0.0 / 0.0 | 2.9 / 0.0 | 55.9 / 0.0 |  | 2.579 |
| d/4MiB k=1 | Cf-q-retain | q1 | 0.0 / 0.0 | 1.0 / 0.0 | 37.6 / 0.0 |  | 2.388 |
| d/4MiB k=1 | Cf-q-retain | q1slot | 0.0 / 0.0 | 1.0 / 0.0 | 48.0 / 0.0 |  | 2.552 |
| c/P5.4 k=1 | Cf-retain | cur | 3.0 / 0.0 | 1.0 / 0.0 | 16.4 / 0.0 |  | 3.530 |
| c/P5.4 k=1 | Cf-retain | curslot | 3.0 / 0.0 | 1.0 / 0.0 | 15.3 / 0.0 |  | 3.114 |
| c/P5.4 k=1 | Cf-retain | q1 | 3.0 / 0.0 | 1.0 / 0.0 | 16.6 / 0.0 |  | 4.059 |
| c/P5.4 k=1 | Cf-retain | q1slot | 3.0 / 0.0 | 1.0 / 0.0 | 16.3 / 0.0 |  | 3.834 |
| c/P5.4 k=1 | Cf-q-retain | cur | 0.0 / 0.0 | 1.0 / 0.0 | 14.5 / 0.0 |  | 3.559 |
| c/P5.4 k=1 | Cf-q-retain | curslot | 0.0 / 0.0 | 1.0 / 0.0 | 13.9 / 0.0 |  | 3.879 |
| c/P5.4 k=1 | Cf-q-retain | q1 | 0.0 / 0.0 | 1.0 / 0.0 | 13.9 / 0.0 |  | 3.781 |
| c/P5.4 k=1 | Cf-q-retain | q1slot | 0.0 / 0.0 | 1.0 / 0.0 | 13.9 / 0.0 |  | 3.544 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 2.061 | 0.020 | 6.321 |  |
| d/16MiB k=1 | Cf-retain | curslot | 2.052 | 0.022 | 6.257 |  |
| d/16MiB k=1 | Cf-retain | q1 | 2.035 | 0.020 | 6.206 |  |
| d/16MiB k=1 | Cf-retain | q1slot | 2.042 | 0.020 | 6.287 |  |
| d/16MiB k=1 | Cf-q-retain | cur | 0.000 | 2.108 | 7.138 |  |
| d/16MiB k=1 | Cf-q-retain | curslot | 0.000 | 2.239 | 7.220 |  |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.000 | 2.091 | 6.688 |  |
| d/16MiB k=1 | Cf-q-retain | q1slot | 0.000 | 2.061 | 7.171 |  |
| d/16MiB k=8 | Cf-retain | cur | 2.742 | 0.009 | 6.792 |  |
| d/16MiB k=8 | Cf-retain | curslot | 2.705 | 0.010 | 6.781 |  |
| d/16MiB k=8 | Cf-retain | q1 | 2.682 | 0.010 | 6.790 |  |
| d/16MiB k=8 | Cf-retain | q1slot | 2.723 | 0.010 | 6.815 |  |
| d/16MiB k=8 | Cf-q-retain | cur | 0.000 | 1.686 | 6.937 |  |
| d/16MiB k=8 | Cf-q-retain | curslot | 0.000 | 1.590 | 6.828 |  |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.000 | 1.670 | 6.939 |  |
| d/16MiB k=8 | Cf-q-retain | q1slot | 0.000 | 1.597 | 6.784 |  |
| d/4MiB k=1 | Cf-retain | cur | 0.515 | 0.015 | 1.528 |  |
| d/4MiB k=1 | Cf-retain | curslot | 0.505 | 0.015 | 1.542 |  |
| d/4MiB k=1 | Cf-retain | q1 | 0.504 | 0.015 | 1.527 |  |
| d/4MiB k=1 | Cf-retain | q1slot | 0.503 | 0.013 | 1.533 |  |
| d/4MiB k=1 | Cf-q-retain | cur | 0.000 | 0.522 | 1.569 |  |
| d/4MiB k=1 | Cf-q-retain | curslot | 0.000 | 0.527 | 1.728 |  |
| d/4MiB k=1 | Cf-q-retain | q1 | 0.000 | 0.496 | 1.580 |  |
| d/4MiB k=1 | Cf-q-retain | q1slot | 0.000 | 0.503 | 1.621 |  |
| c/P5.4 k=1 | Cf-retain | cur | 0.534 | 0.019 | 1.491 |  |
| c/P5.4 k=1 | Cf-retain | curslot | 0.531 | 0.019 | 1.481 |  |
| c/P5.4 k=1 | Cf-retain | q1 | 0.553 | 0.019 | 1.499 |  |
| c/P5.4 k=1 | Cf-retain | q1slot | 0.534 | 0.019 | 1.499 |  |
| c/P5.4 k=1 | Cf-q-retain | cur | 0.000 | 0.513 | 1.516 |  |
| c/P5.4 k=1 | Cf-q-retain | curslot | 0.000 | 0.512 | 1.534 |  |
| c/P5.4 k=1 | Cf-q-retain | q1 | 0.000 | 0.512 | 1.515 |  |
| c/P5.4 k=1 | Cf-q-retain | q1slot | 0.000 | 0.511 | 1.522 |  |

Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A process of the same arm and round):

| workload | cell | arm | per round (CPU) | per round (wall) |
|---|---|---|---|---|
