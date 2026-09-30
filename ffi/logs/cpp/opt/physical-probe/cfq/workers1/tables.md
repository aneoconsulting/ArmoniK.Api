# workers1: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median [p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary context switches per call (median of the processes).

## measure: main figures (every process under AB_ENV, the allocator setting; 3 rounds)

| workload | cell | arm | CPU ms | wall ms | flt | csw | n |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 7.861 [7.715-7.988] | 7.902 [6.944-10.697] | 4.5 | 36.7 | 30 |
| d/16MiB k=1 | Cf-retain | q1 | 7.867 [7.733-7.940] | 8.252 [7.016-8.930] | 4.5 | 32.9 | 30 |
| d/16MiB k=1 | Cf-q-retain | cur | 7.949 [7.888-8.016] | 8.116 [7.338-8.441] | 0.0 | 34.1 | 30 |
| d/16MiB k=1 | Cf-q-retain | q1 | 7.955 [7.884-8.080] | 7.901 [6.829-9.050] | 0.0 | 28.0 | 30 |
| d/16MiB k=8 | Cf-retain | cur | 9.444 [9.270-9.615] | 8.302 [8.081-8.510] | 10.4 | 43.0 | 30 |
| d/16MiB k=8 | Cf-retain | q1 | 9.511 [9.313-9.611] | 8.308 [8.111-8.596] | 16.7 | 44.0 | 30 |
| d/16MiB k=8 | Cf-q-retain | cur | 7.852 [7.773-7.918] | 8.045 [7.778-8.317] | 3.2 | 42.8 | 30 |
| d/16MiB k=8 | Cf-q-retain | q1 | 7.888 [7.800-7.937] | 8.026 [7.759-8.296] | 6.4 | 42.2 | 30 |

Run-queue wait per call (ms; schedstat's second field, every thread of the class; median of the processes) and the batch trace (blocking path: per batch of k calls, median over the batches of the processes, ms): batch wall, one call's duration median [p10-p90], first call start to last call end (span), batch start to its last call's start (dispatch), last call end to batch end (completion).

| workload | cell | arm | wait: caller | main | tokio-rt-worker | event_engine | batch wall | call [p10-p90] | span | dispatch | completion |
|---|---|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 0.000 | 0.000 | 0.000 |  | 7.902 | 7.842 [6.906-8.625] | 7.842 | 0.008 | 0.053 |
| d/16MiB k=1 | Cf-retain | q1 | 0.000 | 0.000 | 0.000 |  | 8.078 | 8.016 [6.992-8.619] | 8.016 | 0.008 | 0.053 |
| d/16MiB k=1 | Cf-q-retain | cur | 0.000 | 0.000 | 0.001 |  | - | - [---] | - | - | - |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-retain | cur | 0.213 | 0.000 | 0.016 |  | 66.341 | 62.835 [56.261-66.564] | 66.321 | 0.943 | 0.011 |
| d/16MiB k=8 | Cf-retain | q1 | 0.181 | 0.000 | 0.050 |  | 66.703 | 63.132 [55.715-66.787] | 66.675 | 0.940 | 0.011 |
| d/16MiB k=8 | Cf-q-retain | cur | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.000 | 0.000 | 0.000 |  | - | - [---] | - | - | - |

Context switches per call by thread class, voluntary / involuntary (median of the processes), and the server during the loop (AK_SERVER_PID: CPU per call, ms; its threads' voluntary switches are not read):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine | server CPU |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 6.6 / 0.0 | 1.0 / 0.0 | 29.1 / 0.0 |  | 11.251 |
| d/16MiB k=1 | Cf-retain | q1 | 6.6 / 0.0 | 1.0 / 0.0 | 25.3 / 0.0 |  | 11.352 |
| d/16MiB k=1 | Cf-q-retain | cur | 0.0 / 0.0 | 9.0 / 0.0 | 25.1 / 0.0 |  | 11.478 |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.0 / 0.0 | 5.5 / 0.0 | 22.4 / 0.0 |  | 11.147 |
| d/16MiB k=8 | Cf-retain | cur | 8.7 / 0.0 | 0.1 / 0.0 | 34.2 / 0.0 |  | 19.907 |
| d/16MiB k=8 | Cf-retain | q1 | 8.7 / 0.0 | 0.1 / 0.0 | 35.1 / 0.0 |  | 19.991 |
| d/16MiB k=8 | Cf-q-retain | cur | 0.0 / 0.0 | 3.6 / 0.0 | 39.2 / 0.0 |  | 19.262 |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.0 / 0.0 | 3.5 / 0.0 | 38.7 / 0.0 |  | 19.247 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 2.056 | 0.020 | 5.781 |  |
| d/16MiB k=1 | Cf-retain | q1 | 2.077 | 0.020 | 5.758 |  |
| d/16MiB k=1 | Cf-q-retain | cur | 0.000 | 2.075 | 5.869 |  |
| d/16MiB k=1 | Cf-q-retain | q1 | 0.000 | 2.055 | 5.920 |  |
| d/16MiB k=8 | Cf-retain | cur | 2.959 | 0.007 | 6.478 |  |
| d/16MiB k=8 | Cf-retain | q1 | 3.012 | 0.008 | 6.487 |  |
| d/16MiB k=8 | Cf-q-retain | cur | 0.000 | 1.582 | 6.281 |  |
| d/16MiB k=8 | Cf-q-retain | q1 | 0.000 | 1.588 | 6.315 |  |

Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A process of the same arm and round):

| workload | cell | arm | per round (CPU) | per round (wall) |
|---|---|---|---|---|
