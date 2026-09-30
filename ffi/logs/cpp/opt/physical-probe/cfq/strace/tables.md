# strace: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median [p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary context switches per call (median of the processes).

## strace: syscalls per call (between the loop's markers, fewer batches)

| workload | cell | arm | calls | socket writes | bytes/write | socket reads | epoll_wait | futex | all syscalls |
|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | cur | 12 | 1026.5 | 16353 | 12.2 | 1022.9 | 47.2 | 2121.2 |
| d/16MiB k=1 | Cf-retain | q1slot | 12 | 1026.4 | 16355 | 11.8 | 1026.8 | 46.3 | 2123.2 |
| d/16MiB k=1 | Cf-q-retain | cur | 12 | 1026.0 | 16361 | 11.8 | 1019.2 | 69.0 | 2144.2 |
| d/16MiB k=1 | Cf-q-retain | q1slot | 12 | 1028.2 | 16327 | 9.8 | 1024.5 | 64.5 | 2143.8 |
| d/16MiB k=8 | Cf-retain | cur | 24 | 1028.8 | 16317 | 9.8 | 975.0 | 51.5 | 2067.7 |
| d/16MiB k=8 | Cf-retain | q1slot | 24 | 1027.7 | 16334 | 9.7 | 997.2 | 56.0 | 2091.5 |
| d/16MiB k=8 | Cf-q-retain | cur | 24 | 1027.8 | 16334 | 9.6 | 996.5 | 79.2 | 2116.4 |
| d/16MiB k=8 | Cf-q-retain | q1slot | 24 | 1028.5 | 16322 | 9.7 | 1008.5 | 78.9 | 2128.0 |
