# workers2: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median [p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary context switches per call (median of the processes).

## measure: main figures (every process under AB_ENV, the allocator setting; 3 rounds)

| workload | cell | arm | CPU ms | wall ms | flt | csw | n |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 8.156 [8.059-8.314] | 8.368 [7.956-8.698] | 4.5 | 99.4 | 30 |
| d/16MiB k=1 | Cf-retain | q1 | 8.155 [8.040-8.237] | 8.564 [8.045-10.144] | 4.5 | 78.6 | 30 |
| d/16MiB k=1 | Cf-q-retain | cur | 8.733 [8.600-9.035] | 8.393 [7.313-10.060] | 0.1 | 203.1 | 30 |
| d/16MiB k=1 | Cf-q-retain | q1 | 8.614 [8.514-8.871] | 8.521 [7.924-9.787] | 0.0 | 172.6 | 30 |
| d/16MiB k=8 | Cf-retain | cur | 9.458 [9.363-9.697] | 8.372 [8.171-8.780] | 16.7 | 67.3 | 30 |
| d/16MiB k=8 | Cf-retain | q1 | 9.521 [9.390-9.725] | 8.352 [8.139-8.588] | 13.6 | 66.7 | 30 |
| d/16MiB k=8 | Cf-q-retain | cur | 8.236 [8.186-9.063] | 8.239 [7.985-8.570] | 3.2 | 79.2 | 30 |
| d/16MiB k=8 | Cf-q-retain | q1 | 8.748 [8.236-9.347] | 8.287 [8.029-8.709] | 6.4 | 130.6 | 30 |

Run-queue wait per call (ms; schedstat's second field, every thread of the class; median of the processes) and the batch trace (blocking path: per batch of k calls, median over the batches of the processes, ms): batch wall, one call's duration median [p10-p90], first call start to last call end (span), batch start to its last call's start (dispatch), last call end to batch end (completion).

| workload | cell | arm | wait: caller | main | tokio-rt-worker | event_engine | batch wall | call [p10-p90] | span | dispatch | completion |
|---|---|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 0.000 | 0.000 | 0.001 |  | 8.395 | 8.336 [7.227-9.030] | 8.336 | 0.008 | 0.053 |
| d/16MiB k=1 | Cf-retain | q1 | 0.000 | 0.000 | 0.001 |  | 8.432 | 8.391 [7.654-9.161] | 8.391 | 0.008 | 0.053 |
| d/16MiB k=1 | Cf-q-retain | cur | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-retain | cur | 0.248 | 0.000 | 0.147 |  | 66.868 | 63.190 [56.675-67.153] | 66.842 | 0.898 | 0.012 |
| d/16MiB k=8 | Cf-retain | q1 | 0.261 | 0.000 | 0.140 |  | 66.622 | 63.276 [56.046-66.595] | 66.596 | 0.909 | 0.012 |
| d/16MiB k=8 | Cf-q-retain | cur | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |

Context switches per call by thread class, voluntary / involuntary (median of the processes), and the server during the loop (AK_SERVER_PID: CPU per call, ms; its threads' voluntary switches are not read):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine | server CPU |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 6.0 / 0.0 | 1.0 / 0.0 | 92.4 / 0.0 |  | 11.617 |
| d/16MiB k=1 | Cf-retain | q1 | 6.0 / 0.0 | 1.0 / 0.0 | 71.5 / 0.0 |  | 11.260 |
| d/16MiB k=1 | Cf-q-retain | cur | 0.0 / 0.0 | 8.8 / 0.0 | 194.2 / 0.0 |  | 11.173 |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.0 / 0.0 | 5.0 / 0.0 | 167.6 / 0.0 |  | 11.544 |
| d/16MiB k=8 | Cf-retain | cur | 8.5 / 0.0 | 0.1 / 0.0 | 58.6 / 0.0 |  | 19.881 |
| d/16MiB k=8 | Cf-retain | q1 | 8.5 / 0.0 | 0.1 / 0.0 | 58.0 / 0.0 |  | 19.856 |
| d/16MiB k=8 | Cf-q-retain | cur | 0.0 / 0.0 | 4.0 / 0.0 | 75.1 / 0.0 |  | 19.823 |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.0 / 0.0 | 4.1 / 0.0 | 126.7 / 0.0 |  | 19.861 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 2.044 | 0.020 | 6.117 |  |
| d/16MiB k=1 | Cf-retain | q1 | 2.047 | 0.021 | 6.081 |  |
| d/16MiB k=1 | Cf-q-retain | cur | 0.000 | 2.092 | 6.733 |  |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.000 | 2.050 | 6.616 |  |
| d/16MiB k=8 | Cf-retain | cur | 2.819 | 0.008 | 6.702 |  |
| d/16MiB k=8 | Cf-retain | q1 | 2.793 | 0.008 | 6.739 |  |
| d/16MiB k=8 | Cf-q-retain | cur | 0.000 | 1.569 | 6.646 |  |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.000 | 1.610 | 7.136 |  |

Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A process of the same arm and round):

| workload | cell | arm | per round (CPU) | per round (wall) |
|---|---|---|---|---|
