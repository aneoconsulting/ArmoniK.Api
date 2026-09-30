# ab: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median [p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary context switches per call (median of the processes).

## measure: main figures (every process under AB_ENV, the allocator setting; 3 rounds)

| workload | cell | arm | CPU ms | wall ms | flt | csw | n |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 8.267 [8.041-8.584] | 7.886 [7.258-9.787] | 4.3 | 99.9 | 30 |
| d/16MiB k=1 | Cf-retain | curslot | 8.173 [8.056-8.614] | 8.233 [7.329-8.987] | 0.3 | 89.0 | 30 |
| d/16MiB k=1 | Cf-retain | q1 | 8.170 [8.056-8.325] | 8.336 [7.409-8.752] | 4.4 | 93.6 | 30 |
| d/16MiB k=1 | Cf-retain | q1slot | 8.244 [8.106-8.499] | 7.921 [7.194-9.570] | 4.3 | 117.4 | 30 |
| d/16MiB k=1 | Cf-q-retain | cur | 9.542 [9.020-9.816] | 8.278 [7.699-8.970] | 0.1 | 273.8 | 30 |
| d/16MiB k=1 | Cf-q-retain | curslot | 9.510 [8.779-9.677] | 8.367 [7.783-8.995] | 0.1 | 257.1 | 30 |
| d/16MiB k=1 | Cf-q-retain | q1 | 9.615 [9.453-9.796] | 8.246 [7.636-9.833] | 0.2 | 235.0 | 30 |
| d/16MiB k=1 | Cf-q-retain | q1slot | 9.031 [8.846-9.555] | 8.419 [7.924-8.725] | 0.1 | 229.9 | 30 |
| d/16MiB k=8 | Cf-retain | cur | 9.575 [9.402-9.835] | 8.274 [8.137-8.576] | 13.4 | 80.9 | 30 |
| d/16MiB k=8 | Cf-retain | curslot | 9.534 [9.369-9.750] | 8.309 [8.121-8.444] | 17.1 | 75.6 | 30 |
| d/16MiB k=8 | Cf-retain | q1 | 9.599 [9.354-9.915] | 8.275 [8.021-8.516] | 16.9 | 74.7 | 30 |
| d/16MiB k=8 | Cf-retain | q1slot | 9.512 [9.362-9.775] | 8.216 [7.992-8.411] | 7.1 | 71.3 | 30 |
| d/16MiB k=8 | Cf-q-retain | cur | 8.445 [8.233-9.215] | 8.180 [7.822-8.346] | 3.4 | 107.7 | 30 |
| d/16MiB k=8 | Cf-q-retain | curslot | 8.400 [8.264-9.032] | 8.067 [7.841-8.313] | 0.1 | 112.6 | 30 |
| d/16MiB k=8 | Cf-q-retain | q1 | 8.455 [8.214-9.190] | 8.018 [7.745-8.296] | 3.4 | 102.1 | 30 |
| d/16MiB k=8 | Cf-q-retain | q1slot | 8.400 [8.278-8.965] | 8.141 [7.834-8.378] | 0.1 | 98.3 | 30 |
| d/4MiB k=1 | Cf-retain | cur | 2.049 [2.037-2.077] | 2.434 [2.292-2.484] | 0.0 | 30.7 | 30 |
| d/4MiB k=1 | Cf-retain | curslot | 2.043 [2.016-2.078] | 2.452 [2.300-2.489] | 0.0 | 32.6 | 30 |
| d/4MiB k=1 | Cf-retain | q1 | 2.093 [2.047-2.144] | 2.484 [2.341-2.539] | 0.0 | 35.7 | 30 |
| d/4MiB k=1 | Cf-retain | q1slot | 2.044 [2.018-2.082] | 2.383 [2.268-2.487] | 0.0 | 32.3 | 30 |
| d/4MiB k=1 | Cf-q-retain | cur | 2.274 [2.216-2.304] | 2.365 [2.287-2.453] | 0.1 | 57.6 | 30 |
| d/4MiB k=1 | Cf-q-retain | curslot | 2.157 [2.062-2.293] | 2.386 [2.255-2.462] | 0.0 | 44.2 | 30 |
| d/4MiB k=1 | Cf-q-retain | q1 | 2.115 [2.081-2.141] | 2.403 [2.215-2.449] | 0.0 | 42.9 | 30 |
| d/4MiB k=1 | Cf-q-retain | q1slot | 2.109 [2.076-2.151] | 2.394 [2.302-2.440] | 0.1 | 40.9 | 30 |
| c/P5.4 k=1 | Cf-retain | cur | 2.014 [1.984-2.068] | 3.227 [2.970-4.146] | 0.0 | 18.5 | 30 |
| c/P5.4 k=1 | Cf-retain | curslot | 2.037 [1.998-2.106] | 3.561 [2.960-4.375] | 0.0 | 20.1 | 30 |
| c/P5.4 k=1 | Cf-retain | q1 | 2.044 [2.011-2.100] | 3.661 [3.194-4.357] | 0.0 | 19.2 | 30 |
| c/P5.4 k=1 | Cf-retain | q1slot | 2.066 [2.030-2.093] | 3.924 [3.277-4.677] | 0.0 | 20.6 | 30 |
| c/P5.4 k=1 | Cf-q-retain | cur | 2.004 [1.983-2.057] | 3.267 [3.032-3.829] | 0.0 | 13.6 | 30 |
| c/P5.4 k=1 | Cf-q-retain | curslot | 2.024 [1.983-2.060] | 3.520 [2.862-4.186] | 0.0 | 15.1 | 30 |
| c/P5.4 k=1 | Cf-q-retain | q1 | 1.977 [1.956-2.016] | 3.025 [2.881-3.577] | 0.0 | 13.2 | 30 |
| c/P5.4 k=1 | Cf-q-retain | q1slot | 2.024 [1.970-2.067] | 3.832 [3.053-4.859] | 0.0 | 13.9 | 30 |

Run-queue wait per call (ms; schedstat's second field, every thread of the class; median of the processes) and the batch trace (blocking path: per batch of k calls, median over the batches of the processes, ms): batch wall, one call's duration median [p10-p90], first call start to last call end (span), batch start to its last call's start (dispatch), last call end to batch end (completion).

| workload | cell | arm | wait: caller | main | tokio-rt-worker | event_engine | batch wall | call [p10-p90] | span | dispatch | completion |
|---|---|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 0.000 | 0.000 | 0.001 |  | 7.927 | 7.867 [7.060-8.652] | 7.867 | 0.008 | 0.053 |
| d/16MiB k=1 | Cf-retain | curslot | 0.000 | 0.000 | 0.001 |  | 7.837 | 7.801 [6.998-8.663] | 7.801 | 0.008 | 0.052 |
| d/16MiB k=1 | Cf-retain | q1 | 0.000 | 0.000 | 0.001 |  | 8.117 | 8.055 [7.025-8.947] | 8.055 | 0.008 | 0.052 |
| d/16MiB k=1 | Cf-retain | q1slot | 0.000 | 0.000 | 0.001 |  | 7.903 | 7.845 [7.088-8.821] | 7.845 | 0.008 | 0.052 |
| d/16MiB k=1 | Cf-q-retain | cur | 0.000 | 0.000 | 0.002 |  | - | - [---] | - | - | - |
| d/16MiB k=1 | Cf-q-retain | curslot | 0.000 | 0.001 | 0.002 |  | - | - [---] | - | - | - |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.002 |  | - | - [---] | - | - | - |
| d/16MiB k=1 | Cf-q-retain | q1slot | 0.000 | 0.000 | 0.002 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-retain | cur | 0.273 | 0.000 | 0.217 |  | 66.512 | 63.163 [55.702-67.121] | 66.485 | 0.870 | 0.049 |
| d/16MiB k=8 | Cf-retain | curslot | 0.283 | 0.000 | 0.214 |  | 66.263 | 63.768 [55.334-66.589] | 66.215 | 0.886 | 0.014 |
| d/16MiB k=8 | Cf-retain | q1 | 0.292 | 0.000 | 0.239 |  | 65.748 | 63.247 [55.404-66.600] | 65.721 | 0.885 | 0.014 |
| d/16MiB k=8 | Cf-retain | q1slot | 0.262 | 0.000 | 0.215 |  | 65.213 | 62.722 [55.554-65.485] | 65.185 | 0.871 | 0.018 |
| d/16MiB k=8 | Cf-q-retain | cur | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-q-retain | curslot | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-q-retain | q1slot | 0.000 | 0.000 | 0.002 |  | - | - [---] | - | - | - |
| d/4MiB k=1 | Cf-retain | cur | 0.000 | 0.000 | 0.000 |  | 2.412 | 2.387 [2.104-2.566] | 2.387 | 0.007 | 0.009 |
| d/4MiB k=1 | Cf-retain | curslot | 0.000 | 0.000 | 0.000 |  | 2.427 | 2.384 [2.096-2.564] | 2.384 | 0.008 | 0.051 |
| d/4MiB k=1 | Cf-retain | q1 | 0.000 | 0.000 | 0.000 |  | 2.477 | 2.415 [2.173-2.572] | 2.415 | 0.008 | 0.052 |
| d/4MiB k=1 | Cf-retain | q1slot | 0.000 | 0.000 | 0.000 |  | 2.389 | 2.362 [2.082-2.533] | 2.362 | 0.007 | 0.014 |
| d/4MiB k=1 | Cf-q-retain | cur | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/4MiB k=1 | Cf-q-retain | curslot | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| d/4MiB k=1 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| d/4MiB k=1 | Cf-q-retain | q1slot | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| c/P5.4 k=1 | Cf-retain | cur | 0.000 | 0.000 | 0.000 |  | 2.909 | 2.853 [2.731-4.599] | 2.853 | 0.008 | 0.072 |
| c/P5.4 k=1 | Cf-retain | curslot | 0.000 | 0.000 | 0.000 |  | 3.138 | 3.057 [2.745-5.998] | 3.057 | 0.008 | 0.072 |
| c/P5.4 k=1 | Cf-retain | q1 | 0.000 | 0.000 | 0.000 |  | 3.028 | 2.947 [2.728-5.356] | 2.947 | 0.008 | 0.070 |
| c/P5.4 k=1 | Cf-retain | q1slot | 0.000 | 0.000 | 0.000 |  | 3.230 | 3.162 [2.762-6.369] | 3.162 | 0.008 | 0.053 |
| c/P5.4 k=1 | Cf-q-retain | cur | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| c/P5.4 k=1 | Cf-q-retain | curslot | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| c/P5.4 k=1 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| c/P5.4 k=1 | Cf-q-retain | q1slot | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |

Context switches per call by thread class, voluntary / involuntary (median of the processes), and the server during the loop (AK_SERVER_PID: CPU per call, ms; its threads' voluntary switches are not read):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine | server CPU |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 6.1 / 0.0 | 1.0 / 0.0 | 92.9 / 0.0 |  | 10.716 |
| d/16MiB k=1 | Cf-retain | curslot | 6.0 / 0.0 | 1.0 / 0.0 | 82.0 / 0.0 |  | 10.925 |
| d/16MiB k=1 | Cf-retain | q1 | 6.0 / 0.0 | 1.0 / 0.0 | 86.6 / 0.0 |  | 11.207 |
| d/16MiB k=1 | Cf-retain | q1slot | 6.0 / 0.0 | 1.0 / 0.0 | 110.3 / 0.0 |  | 10.640 |
| d/16MiB k=1 | Cf-q-retain | cur | 0.0 / 0.0 | 8.9 / 0.0 | 264.9 / 0.0 |  | 10.801 |
| d/16MiB k=1 | Cf-q-retain | curslot | 0.0 / 0.0 | 8.9 / 0.0 | 248.1 / 0.0 |  | 10.823 |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.0 / 0.0 | 5.3 / 0.0 | 229.8 / 0.0 |  | 10.026 |
| d/16MiB k=1 | Cf-q-retain | q1slot | 0.0 / 0.0 | 5.2 / 0.0 | 224.7 / 0.0 |  | 10.832 |
| d/16MiB k=8 | Cf-retain | cur | 8.6 / 0.0 | 0.1 / 0.0 | 72.0 / 0.1 |  | 19.839 |
| d/16MiB k=8 | Cf-retain | curslot | 8.6 / 0.1 | 0.1 / 0.0 | 66.8 / 0.0 |  | 19.858 |
| d/16MiB k=8 | Cf-retain | q1 | 8.6 / 0.1 | 0.1 / 0.0 | 65.9 / 0.0 |  | 19.822 |
| d/16MiB k=8 | Cf-retain | q1slot | 8.6 / 0.1 | 0.1 / 0.0 | 62.5 / 0.0 |  | 19.477 |
| d/16MiB k=8 | Cf-q-retain | cur | 0.0 / 0.0 | 4.1 / 0.0 | 103.8 / 0.0 |  | 19.483 |
| d/16MiB k=8 | Cf-q-retain | curslot | 0.0 / 0.0 | 4.2 / 0.0 | 108.4 / 0.0 |  | 19.547 |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.0 / 0.0 | 4.1 / 0.0 | 98.2 / 0.0 |  | 19.279 |
| d/16MiB k=8 | Cf-q-retain | q1slot | 0.0 / 0.0 | 4.1 / 0.0 | 94.3 / 0.0 |  | 19.316 |
| d/4MiB k=1 | Cf-retain | cur | 2.0 / 0.0 | 1.0 / 0.0 | 27.7 / 0.0 |  | 2.523 |
| d/4MiB k=1 | Cf-retain | curslot | 2.0 / 0.0 | 1.0 / 0.0 | 29.6 / 0.0 |  | 2.503 |
| d/4MiB k=1 | Cf-retain | q1 | 2.0 / 0.0 | 1.0 / 0.0 | 32.7 / 0.0 |  | 2.521 |
| d/4MiB k=1 | Cf-retain | q1slot | 2.0 / 0.0 | 1.0 / 0.0 | 29.3 / 0.0 |  | 2.477 |
| d/4MiB k=1 | Cf-q-retain | cur | 0.0 / 0.0 | 2.9 / 0.0 | 54.6 / 0.0 |  | 2.470 |
| d/4MiB k=1 | Cf-q-retain | curslot | 0.0 / 0.0 | 3.0 / 0.0 | 41.2 / 0.0 |  | 2.538 |
| d/4MiB k=1 | Cf-q-retain | q1 | 0.0 / 0.0 | 1.0 / 0.0 | 41.9 / 0.0 |  | 2.565 |
| d/4MiB k=1 | Cf-q-retain | q1slot | 0.0 / 0.0 | 1.0 / 0.0 | 39.9 / 0.0 |  | 2.530 |
| c/P5.4 k=1 | Cf-retain | cur | 3.0 / 0.0 | 1.0 / 0.0 | 14.5 / 0.0 |  | 2.789 |
| c/P5.4 k=1 | Cf-retain | curslot | 3.0 / 0.0 | 1.0 / 0.0 | 16.1 / 0.0 |  | 3.495 |
| c/P5.4 k=1 | Cf-retain | q1 | 3.0 / 0.0 | 1.0 / 0.0 | 15.2 / 0.0 |  | 3.188 |
| c/P5.4 k=1 | Cf-retain | q1slot | 3.0 / 0.0 | 1.0 / 0.0 | 16.6 / 0.0 |  | 3.675 |
| c/P5.4 k=1 | Cf-q-retain | cur | 0.0 / 0.0 | 1.0 / 0.0 | 12.6 / 0.0 |  | 3.013 |
| c/P5.4 k=1 | Cf-q-retain | curslot | 0.0 / 0.0 | 1.0 / 0.0 | 14.1 / 0.0 |  | 3.325 |
| c/P5.4 k=1 | Cf-q-retain | q1 | 0.0 / 0.0 | 1.0 / 0.0 | 12.2 / 0.0 |  | 2.760 |
| c/P5.4 k=1 | Cf-q-retain | q1slot | 0.0 / 0.0 | 1.0 / 0.0 | 12.9 / 0.0 |  | 3.374 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 2.037 | 0.022 | 6.143 |  |
| d/16MiB k=1 | Cf-retain | curslot | 2.065 | 0.022 | 6.061 |  |
| d/16MiB k=1 | Cf-retain | q1 | 2.068 | 0.022 | 6.131 |  |
| d/16MiB k=1 | Cf-retain | q1slot | 2.037 | 0.022 | 6.151 |  |
| d/16MiB k=1 | Cf-q-retain | cur | 0.000 | 2.239 | 7.356 |  |
| d/16MiB k=1 | Cf-q-retain | curslot | 0.000 | 2.179 | 7.243 |  |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.000 | 2.227 | 7.343 |  |
| d/16MiB k=1 | Cf-q-retain | q1slot | 0.000 | 2.100 | 6.973 |  |
| d/16MiB k=8 | Cf-retain | cur | 2.728 | 0.010 | 6.835 |  |
| d/16MiB k=8 | Cf-retain | curslot | 2.742 | 0.009 | 6.781 |  |
| d/16MiB k=8 | Cf-retain | q1 | 2.772 | 0.009 | 6.811 |  |
| d/16MiB k=8 | Cf-retain | q1slot | 2.706 | 0.010 | 6.761 |  |
| d/16MiB k=8 | Cf-q-retain | cur | 0.000 | 1.617 | 6.844 |  |
| d/16MiB k=8 | Cf-q-retain | curslot | 0.000 | 1.592 | 6.920 |  |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.000 | 1.588 | 6.808 |  |
| d/16MiB k=8 | Cf-q-retain | q1slot | 0.000 | 1.599 | 6.899 |  |
| d/4MiB k=1 | Cf-retain | cur | 0.501 | 0.013 | 1.539 |  |
| d/4MiB k=1 | Cf-retain | curslot | 0.501 | 0.015 | 1.525 |  |
| d/4MiB k=1 | Cf-retain | q1 | 0.511 | 0.018 | 1.568 |  |
| d/4MiB k=1 | Cf-retain | q1slot | 0.500 | 0.015 | 1.531 |  |
| d/4MiB k=1 | Cf-q-retain | cur | 0.000 | 0.539 | 1.729 |  |
| d/4MiB k=1 | Cf-q-retain | curslot | 0.000 | 0.522 | 1.647 |  |
| d/4MiB k=1 | Cf-q-retain | q1 | 0.000 | 0.508 | 1.604 |  |
| d/4MiB k=1 | Cf-q-retain | q1slot | 0.000 | 0.499 | 1.613 |  |
| c/P5.4 k=1 | Cf-retain | cur | 0.536 | 0.019 | 1.448 |  |
| c/P5.4 k=1 | Cf-retain | curslot | 0.535 | 0.019 | 1.500 |  |
| c/P5.4 k=1 | Cf-retain | q1 | 0.555 | 0.019 | 1.483 |  |
| c/P5.4 k=1 | Cf-retain | q1slot | 0.542 | 0.019 | 1.503 |  |
| c/P5.4 k=1 | Cf-q-retain | cur | 0.000 | 0.507 | 1.495 |  |
| c/P5.4 k=1 | Cf-q-retain | curslot | 0.000 | 0.507 | 1.531 |  |
| c/P5.4 k=1 | Cf-q-retain | q1 | 0.000 | 0.510 | 1.469 |  |
| c/P5.4 k=1 | Cf-q-retain | q1slot | 0.000 | 0.511 | 1.514 |  |

Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A process of the same arm and round):

| workload | cell | arm | per round (CPU) | per round (wall) |
|---|---|---|---|---|
