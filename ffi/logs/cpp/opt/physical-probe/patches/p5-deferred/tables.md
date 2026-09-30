# p5-deferred: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median [p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary context switches per call (median of the processes).

## measure: main figures (every process under AB_ENV, the allocator setting; 3 rounds)

| workload | cell | arm | CPU ms | wall ms | flt | csw | n |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | p5 | 8.200 [8.084-8.250] | 8.189 [8.056-8.798] | 0.1 | 225.8 | 30 |
| d/16MiB k=1 | D-retain | p5 | 8.269 [8.171-8.423] | 8.265 [8.152-9.251] | 0.1 | 220.2 | 30 |
| d/16MiB k=1 | Cf-retain | p5 | 8.328 [8.032-8.966] | 8.437 [7.458-9.657] | 0.1 | 104.9 | 30 |
| d/16MiB k=1 | Cf-enc-retain | p5 | 7.942 [7.759-8.058] | 9.033 [8.350-9.329] | 0.1 | 119.0 | 30 |
| d/16MiB k=1 | Cf-encp-retain | p5 | 7.768 [7.637-7.936] | 9.112 [8.483-11.257] | 0.1 | 86.2 | 30 |
| d/16MiB k=8 | A | p5 | 9.561 [9.446-9.685] | 7.531 [7.332-7.676] | 1.8 | 192.5 | 30 |
| d/16MiB k=8 | D-retain | p5 | 9.580 [9.444-9.809] | 7.479 [7.225-7.667] | 0.1 | 192.1 | 30 |
| d/16MiB k=8 | Cf-retain | p5 | 9.581 [9.417-9.743] | 8.278 [8.140-8.437] | 0.2 | 76.8 | 30 |
| d/16MiB k=8 | Cf-enc-retain | p5 | 9.142 [8.966-9.397] | 8.603 [8.415-8.834] | 0.2 | 83.7 | 30 |
| d/16MiB k=8 | Cf-encp-retain | p5 | 9.104 [8.861-9.304] | 8.653 [8.415-8.843] | 0.2 | 89.3 | 30 |
| d/4MiB k=1 | A | p5 | 2.113 [2.069-2.137] | 2.238 [2.204-2.269] | 0.0 | 60.7 | 30 |
| d/4MiB k=1 | D-retain | p5 | 2.155 [2.125-2.178] | 2.295 [2.263-2.315] | 0.0 | 58.1 | 30 |
| d/4MiB k=1 | Cf-retain | p5 | 2.068 [2.024-2.095] | 2.348 [2.251-2.471] | 0.0 | 32.4 | 30 |
| d/4MiB k=1 | Cf-enc-retain | p5 | 1.991 [1.962-2.017] | 2.520 [2.476-2.558] | 0.0 | 19.0 | 30 |
| d/4MiB k=1 | Cf-encp-retain | p5 | 1.982 [1.926-2.162] | 2.517 [2.439-2.582] | 0.1 | 24.6 | 30 |
| d/4MiB k=8 | A | p5 | 2.577 [2.545-2.639] | 2.116 [1.974-2.191] | 0.0 | 53.1 | 30 |
| d/4MiB k=8 | D-retain | p5 | 2.591 [2.554-2.628] | 2.073 [1.967-2.167] | 0.1 | 54.7 | 30 |
| d/4MiB k=8 | Cf-retain | p5 | 2.586 [2.540-2.650] | 2.274 [2.194-2.342] | 0.2 | 20.0 | 30 |
| d/4MiB k=8 | Cf-enc-retain | p5 | 2.411 [2.365-2.479] | 2.187 [2.103-2.306] | 0.1 | 33.4 | 30 |
| d/4MiB k=8 | Cf-encp-retain | p5 | 2.400 [2.366-2.454] | 2.162 [2.081-2.237] | 0.1 | 35.7 | 30 |
| c/P5.4 k=1 | A | p5 | 2.197 [2.143-2.712] | 3.415 [2.907-6.969] | 0.0 | 45.5 | 30 |
| c/P5.4 k=1 | D-retain | p5 | 2.200 [2.158-2.735] | 3.456 [3.077-7.002] | 0.0 | 47.2 | 30 |
| c/P5.4 k=1 | Cf-retain | p5 | 2.008 [1.971-2.047] | 3.178 [2.937-3.626] | 0.0 | 19.6 | 30 |
| c/P5.4 k=8 | A | p5 | 2.708 [2.642-2.768] | 3.077 [2.944-3.319] | 2.0 | 50.7 | 30 |
| c/P5.4 k=8 | D-retain | p5 | 2.707 [2.584-2.768] | 3.109 [2.931-3.231] | 0.1 | 52.9 | 30 |
| c/P5.4 k=8 | Cf-retain | p5 | 2.575 [2.507-2.618] | 2.606 [2.475-2.717] | 0.2 | 21.5 | 30 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | p5 | 2.104 | 0.017 | 6.283 |  |
| d/16MiB k=1 | Cf-enc-retain | p5 | 0.132 | 0.018 | 7.812 |  |
| d/16MiB k=1 | Cf-encp-retain | p5 | 0.112 | 0.018 | 7.685 |  |
| d/16MiB k=8 | Cf-retain | p5 | 2.703 | 0.007 | 6.896 |  |
| d/16MiB k=8 | Cf-enc-retain | p5 | 0.150 | 0.007 | 9.054 |  |
| d/16MiB k=8 | Cf-encp-retain | p5 | 0.131 | 0.007 | 8.957 |  |
| d/4MiB k=1 | Cf-retain | p5 | 0.501 | 0.012 | 1.554 |  |
| d/4MiB k=1 | Cf-enc-retain | p5 | 0.057 | 0.015 | 1.910 |  |
| d/4MiB k=1 | Cf-encp-retain | p5 | 0.041 | 0.014 | 1.973 |  |
| d/4MiB k=8 | Cf-retain | p5 | 0.869 | 0.005 | 1.720 |  |
| d/4MiB k=8 | Cf-enc-retain | p5 | 0.059 | 0.005 | 2.349 |  |
| d/4MiB k=8 | Cf-encp-retain | p5 | 0.048 | 0.005 | 2.353 |  |
| c/P5.4 k=1 | Cf-retain | p5 | 0.545 | 0.018 | 1.453 |  |
| c/P5.4 k=8 | Cf-retain | p5 | 0.891 | 0.006 | 1.674 |  |

Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A process of the same arm and round):

| workload | cell | arm | per round (CPU) | per round (wall) |
|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | p5 | +0.713 +0.174 -0.162 | +0.404 -0.338 +0.253 |
| d/16MiB k=1 | Cf-enc-retain | p5 | -0.194 -0.378 -0.239 | +0.542 +0.930 +0.192 |
| d/16MiB k=1 | Cf-encp-retain | p5 | -0.498 -0.418 -0.341 | +2.261 +0.794 +0.768 |
| d/16MiB k=8 | Cf-retain | p5 | +0.035 -0.002 +0.004 | +0.685 +0.730 +0.808 |
| d/16MiB k=8 | Cf-enc-retain | p5 | -0.332 -0.490 -0.352 | +1.028 +1.074 +1.130 |
| d/16MiB k=8 | Cf-encp-retain | p5 | -0.433 -0.495 -0.441 | +1.086 +1.101 +1.070 |
| d/4MiB k=1 | Cf-retain | p5 | -0.043 -0.035 -0.056 | +0.089 +0.182 +0.109 |
| d/4MiB k=1 | Cf-enc-retain | p5 | -0.064 -0.133 -0.155 | +0.268 +0.296 +0.298 |
| d/4MiB k=1 | Cf-encp-retain | p5 | -0.094 -0.113 -0.192 | +0.253 +0.274 +0.281 |
| d/4MiB k=8 | Cf-retain | p5 | +0.007 +0.004 +0.012 | +0.173 +0.179 +0.183 |
| d/4MiB k=8 | Cf-enc-retain | p5 | -0.128 -0.205 -0.166 | +0.112 +0.091 +0.062 |
| d/4MiB k=8 | Cf-encp-retain | p5 | -0.173 -0.190 -0.169 | +0.087 +0.065 +0.021 |
| c/P5.4 k=1 | Cf-retain | p5 | -0.142 -0.696 -0.193 | +0.125 -3.584 -0.212 |
| c/P5.4 k=8 | Cf-retain | p5 | -0.143 -0.160 -0.159 | -0.604 -0.377 -0.506 |

## default: the default-allocator pass (1 round)

| workload | cell | arm | CPU ms | wall ms | flt | csw | n |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | p5 | 8.475 [8.286-8.664] | 9.307 [8.412-10.372] | 0.1 | 243.7 | 10 |
| d/16MiB k=1 | D-retain | p5 | 8.442 [8.262-8.533] | 9.289 [8.700-9.777] | 0.1 | 211.8 | 10 |
| d/16MiB k=1 | Cf-retain | p5 | 8.189 [8.113-8.321] | 9.296 [8.877-9.989] | 0.0 | 90.2 | 10 |
| d/16MiB k=1 | Cf-enc-retain | p5 | 8.169 [7.972-8.448] | 10.232 [9.526-10.733] | 0.0 | 95.5 | 10 |
| d/16MiB k=1 | Cf-encp-retain | p5 | 8.043 [7.899-8.182] | 10.226 [9.745-10.534] | 0.0 | 86.7 | 10 |
| d/16MiB k=8 | A | p5 | 9.400 [9.314-9.484] | 7.333 [7.268-7.496] | 1.7 | 182.4 | 10 |
| d/16MiB k=8 | D-retain | p5 | 9.597 [9.391-9.818] | 7.509 [7.345-7.673] | 0.1 | 185.8 | 10 |
| d/16MiB k=8 | Cf-retain | p5 | 9.507 [9.405-9.553] | 8.441 [8.250-8.555] | 0.3 | 72.1 | 10 |
| d/16MiB k=8 | Cf-enc-retain | p5 | 9.154 [9.065-9.266] | 8.721 [8.425-8.973] | 0.2 | 88.8 | 10 |
| d/16MiB k=8 | Cf-encp-retain | p5 | 9.112 [8.923-9.295] | 8.465 [8.165-8.631] | 0.2 | 91.2 | 10 |
| d/4MiB k=1 | A | p5 | 2.101 [2.087-2.112] | 2.230 [2.213-2.243] | 0.0 | 59.4 | 10 |
| d/4MiB k=1 | D-retain | p5 | 2.167 [2.154-2.176] | 2.314 [2.278-2.323] | 0.0 | 55.6 | 10 |
| d/4MiB k=1 | Cf-retain | p5 | 2.115 [2.067-2.164] | 2.307 [2.237-2.566] | 0.1 | 34.1 | 10 |
| d/4MiB k=1 | Cf-enc-retain | p5 | 1.977 [1.963-2.043] | 2.562 [2.458-2.607] | 0.1 | 21.9 | 10 |
| d/4MiB k=1 | Cf-encp-retain | p5 | 1.920 [1.900-1.959] | 2.566 [2.531-2.600] | 0.0 | 19.1 | 10 |
| d/4MiB k=8 | A | p5 | 2.588 [2.512-2.608] | 2.132 [1.985-2.169] | 0.1 | 53.3 | 10 |
| d/4MiB k=8 | D-retain | p5 | 2.557 [2.531-2.609] | 2.108 [2.054-2.200] | 0.1 | 53.7 | 10 |
| d/4MiB k=8 | Cf-retain | p5 | 2.581 [2.524-2.664] | 2.252 [2.215-2.369] | 0.1 | 21.5 | 10 |
| d/4MiB k=8 | Cf-enc-retain | p5 | 2.401 [2.362-2.505] | 2.168 [2.124-2.219] | 0.1 | 33.3 | 10 |
| d/4MiB k=8 | Cf-encp-retain | p5 | 2.334 [2.305-2.384] | 2.140 [2.040-2.259] | 0.1 | 33.4 | 10 |
| c/P5.4 k=1 | A | p5 | 2.645 [2.512-2.688] | 6.784 [5.733-6.940] | 0.0 | 109.1 | 10 |
| c/P5.4 k=1 | D-retain | p5 | 2.233 [2.208-2.286] | 3.692 [3.333-4.041] | 0.0 | 51.2 | 10 |
| c/P5.4 k=1 | Cf-retain | p5 | 2.036 [1.993-2.053] | 4.044 [3.555-4.578] | 0.0 | 21.3 | 10 |
| c/P5.4 k=8 | A | p5 | 2.754 [2.717-2.792] | 3.151 [3.088-3.404] | 2.0 | 54.6 | 10 |
| c/P5.4 k=8 | D-retain | p5 | 2.728 [2.640-2.780] | 3.089 [2.886-3.228] | 0.1 | 52.1 | 10 |
| c/P5.4 k=8 | Cf-retain | p5 | 2.553 [2.481-2.639] | 2.610 [2.492-2.771] | 0.2 | 21.3 | 10 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | p5 | 2.032 | 0.018 | 6.159 |  |
| d/16MiB k=1 | Cf-enc-retain | p5 | 0.145 | 0.019 | 8.040 |  |
| d/16MiB k=1 | Cf-encp-retain | p5 | 0.115 | 0.018 | 7.930 |  |
| d/16MiB k=8 | Cf-retain | p5 | 2.674 | 0.007 | 6.812 |  |
| d/16MiB k=8 | Cf-enc-retain | p5 | 0.150 | 0.007 | 9.029 |  |
| d/16MiB k=8 | Cf-encp-retain | p5 | 0.134 | 0.007 | 8.962 |  |
| d/4MiB k=1 | Cf-retain | p5 | 0.525 | 0.014 | 1.580 |  |
| d/4MiB k=1 | Cf-enc-retain | p5 | 0.057 | 0.014 | 1.927 |  |
| d/4MiB k=1 | Cf-encp-retain | p5 | 0.040 | 0.014 | 1.869 |  |
| d/4MiB k=8 | Cf-retain | p5 | 0.862 | 0.005 | 1.717 |  |
| d/4MiB k=8 | Cf-enc-retain | p5 | 0.059 | 0.005 | 2.351 |  |
| d/4MiB k=8 | Cf-encp-retain | p5 | 0.048 | 0.005 | 2.293 |  |
| c/P5.4 k=1 | Cf-retain | p5 | 0.555 | 0.018 | 1.455 |  |
| c/P5.4 k=8 | Cf-retain | p5 | 0.875 | 0.006 | 1.673 |  |

Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A process of the same arm and round):

| workload | cell | arm | per round (CPU) | per round (wall) |
|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | p5 | -0.285 | -0.010 |
| d/16MiB k=1 | Cf-enc-retain | p5 | -0.306 | +0.925 |
| d/16MiB k=1 | Cf-encp-retain | p5 | -0.431 | +0.919 |
| d/16MiB k=8 | Cf-retain | p5 | +0.107 | +1.108 |
| d/16MiB k=8 | Cf-enc-retain | p5 | -0.246 | +1.388 |
| d/16MiB k=8 | Cf-encp-retain | p5 | -0.288 | +1.132 |
| d/4MiB k=1 | Cf-retain | p5 | +0.014 | +0.077 |
| d/4MiB k=1 | Cf-enc-retain | p5 | -0.124 | +0.332 |
| d/4MiB k=1 | Cf-encp-retain | p5 | -0.181 | +0.336 |
| d/4MiB k=8 | Cf-retain | p5 | -0.008 | +0.120 |
| d/4MiB k=8 | Cf-enc-retain | p5 | -0.187 | +0.037 |
| d/4MiB k=8 | Cf-encp-retain | p5 | -0.254 | +0.008 |
| c/P5.4 k=1 | Cf-retain | p5 | -0.609 | -2.740 |
| c/P5.4 k=8 | Cf-retain | p5 | -0.201 | -0.541 |

## strace: syscalls per call (between the loop's markers, fewer batches)

| workload | cell | arm | calls | socket writes | bytes/write | socket reads | epoll_wait | futex | all syscalls |
|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | p5 | 12 | 49.1 | 342004 | 46.2 | 67.8 | 187.8 | 366.2 |
| d/16MiB k=1 | D-retain | p5 | 12 | 49.5 | 339125 | 47.8 | 69.8 | 191.3 | 374.9 |
| d/16MiB k=1 | Cf-retain | p5 | 12 | 1026.6 | 16352 | 11.7 | 1028.2 | 47.2 | 2125.8 |
| d/16MiB k=1 | Cf-enc-retain | p5 | 12 | 1027.8 | 16333 | 13.1 | 1028.4 | 51.0 | 2130.2 |
| d/16MiB k=1 | Cf-encp-retain | p5 | 12 | 1026.8 | 16349 | 13.2 | 1024.1 | 47.3 | 2121.7 |
| d/16MiB k=8 | A | p5 | 24 | 34.5 | 486571 | 35.7 | 51.7 | 156.5 | 283.2 |
| d/16MiB k=8 | D-retain | p5 | 24 | 36.1 | 464684 | 35.9 | 53.3 | 162.3 | 293.2 |
| d/16MiB k=8 | Cf-retain | p5 | 24 | 1028.1 | 16328 | 10.0 | 998.3 | 54.1 | 2092.4 |
| d/16MiB k=8 | Cf-enc-retain | p5 | 24 | 1027.3 | 16341 | 9.8 | 998.0 | 59.8 | 2096.5 |
| d/16MiB k=8 | Cf-encp-retain | p5 | 24 | 1028.2 | 16327 | 9.7 | 996.4 | 57.8 | 2093.3 |
| d/4MiB k=1 | A | p5 | 40 | 14.5 | 289429 | 13.5 | 20.8 | 60.7 | 115.6 |
| d/4MiB k=1 | D-retain | p5 | 40 | 14.2 | 295544 | 13.2 | 20.3 | 59.6 | 113.0 |
| d/4MiB k=1 | Cf-retain | p5 | 40 | 258.1 | 16259 | 4.2 | 262.1 | 24.8 | 554.9 |
| d/4MiB k=1 | Cf-enc-retain | p5 | 40 | 258.1 | 16263 | 4.2 | 261.4 | 23.2 | 551.0 |
| d/4MiB k=1 | Cf-encp-retain | p5 | 40 | 258.4 | 16244 | 4.0 | 261.9 | 21.6 | 550.1 |
| d/4MiB k=8 | A | p5 | 64 | 10.5 | 399094 | 9.8 | 15.6 | 54.1 | 92.5 |
| d/4MiB k=8 | D-retain | p5 | 64 | 11.5 | 365926 | 10.2 | 15.9 | 55.0 | 95.3 |
| d/4MiB k=8 | Cf-retain | p5 | 64 | 256.5 | 16360 | 2.8 | 250.6 | 18.0 | 528.8 |
| d/4MiB k=8 | Cf-enc-retain | p5 | 64 | 256.5 | 16361 | 2.7 | 245.4 | 24.0 | 530.0 |
| d/4MiB k=8 | Cf-encp-retain | p5 | 64 | 256.5 | 16362 | 2.7 | 242.3 | 21.3 | 524.2 |
| c/P5.4 k=1 | A | p5 | 40 | 10.0 | 420727 | 11.4 | 16.4 | 48.8 | 88.6 |
| c/P5.4 k=1 | D-retain | p5 | 40 | 13.2 | 318539 | 13.0 | 15.6 | 53.0 | 96.8 |
| c/P5.4 k=1 | Cf-retain | p5 | 40 | 258.0 | 16266 | 3.4 | 262.4 | 23.1 | 549.9 |
| c/P5.4 k=8 | A | p5 | 48 | 15.1 | 277471 | 12.2 | 15.4 | 56.8 | 100.3 |
| c/P5.4 k=8 | D-retain | p5 | 48 | 13.9 | 301112 | 11.6 | 14.7 | 51.1 | 92.0 |
| c/P5.4 k=8 | Cf-retain | p5 | 48 | 256.4 | 16367 | 3.0 | 252.5 | 16.2 | 528.7 |
