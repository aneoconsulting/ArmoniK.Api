# C# baseline (optimisation pass): core grid, absolute times per case

CONTAINER INSTRUMENTATION, not a result (README 1.1); NOT GATED (gen/gate.sh not run; every codec process ran Cases.Verify, byte identity of every encode arm and variant and every U-* row, before timing; every RPC call checked). One `gen/opt_bench.sh` run, one launch, short BenchmarkDotNet settings (header below). No ratio is formed here.

Notation: each entry is the **median over the BDN actual iterations (rounds) of one case** with **[min-max]** across them; one round = one BDN iteration; per-op value = that iteration's CPU (or wall) / its operations. ` *` = (max - min) > 25 % of the median; ` ^` = round 1 is the slowest and > 10 % above the median (a warm-up tail). Every case ran in its own child process (BDN default toolchain). Allocation and GC: BenchmarkDotNet MemoryDiagnoser, one extra workload iteration per case after the actual stage, GC.GetTotalAllocatedBytes(precise) over every thread / its operations; Gen0/1/2 = collections during that iteration per 1,000 operations (with the operation count beside it). Crossing counts fwd/rev/grow/reset per operation are from the committed count files (`gen/counts.txt`, `gen/counts-nounk.txt`, `gen/rpc-counts.txt`, gated; none measured in this run).

CPUs seen by each case's own process (affinity mask, Environment.ProcessorCount, single-CPU override), over every case: (2, 2, False).

```
# csharp slice OPTIMISATION BENCHMARK (gen/opt_bench.sh), core grid (AK_CAMPAIGN_GRID=core, CAMPAIGN section 4.0)
# CONTAINER INSTRUMENTATION: not a campaign result (README 1.1, CAMPAIGN section 2); NOT GATED (gen/gate.sh not run); each codec process's Cases.Verify byte-identity pre-check is on, and every RPC call is checked (req 18)
# commit:     24a92944bca8cf5fe9ff3ee54c36c5175928d31c (clean)
# utc:        2026-10-04T08:49:04Z
# machine:    Intel(R) Xeon(R) Processor @ 2.80GHz; 4 logical CPUs; kernel 6.18.44-fc-v64; smt 0
# cpu sets:   CLIENT=0,1 SERVER=2,3 (taskset); AK_WORKERS=8 (core runtime workers, .NET thread pool min/max in the RPC client), server tokio workers AK_SERVER_THREADS=8
# .NET:       SDK 8.0.131; runtimes Microsoft.AspNetCore.App 8.0.31;Microsoft.NETCore.App 8.0.31; (Ubuntu noble-updates packages dotnet-sdk-8.0 8.0.131-0ubuntu1~24.04.1); target net8.0 Release; workstation concurrent GC, tiering/PGO at defaults
# rust:       rustc 1.94.1 (e408947bf 2026-03-25); cargo 1.94.1 (29ea6fb6a 2026-03-24)
# core:       target-core (rpc,init-guard) sha256 8932d20505b6d5f062a59e7a830a1df03f9705bfe8b3e5e63ef0296d5bd9b436
#             target-core-nounk (rpc,init-guard, --no-default-features) sha256 837687368933381fda3ca67cd0747072257c736a1cff029e4af5d4f483df5631
#             target-core-h2b (rpc,init-guard, h2-batch) sha256 a6faa4d6d8aacbf3f557b88e2ca6f01f2a3fdc8d19a08f22c92630288a12d53a
#             built by gen/build_core.sh from git archive HEAD ffi/poc/codec (last core commit fd69b0d6)
# BDN:        BenchmarkDotNet 0.15.8, DEFAULT toolchain (one child process per case, as the campaign), one launch (launch 1: full build first), merged runs (one per codec build; one per RPC run kind)
#             codec: --warmup 25 --rounds 6 --iteration-ms 40 (campaign: 10 / 5 / 100, 3 launches)
#             rpc:   --warmup 10 --rounds 6 --iteration-ms 100 (campaign: 10 / 5 / 100, 3 launches); >= 45 calls per caller thread before the first measured value (req 24: >= 20)
#             warm-up length (JOURNAL 73, logs/csharp/opt/tier-check/): .NET 8 tiers up after a call-counting delay of 100 ms (x10 on a one-CPU affinity mask), restarted by each new tier-0 JIT; a BDN child measured tier-0 or mid-tier-up code with 4 x 40 ms of warm-up (1 CPU: always; 2 CPUs: often), and settled code with 10 x 100 ms or 25 x 40 ms on 2 CPUs; every timed process refuses a one-CPU affinity mask (CpuGuard) and each case's first row carries the CPUs its process saw
#             MemoryDiagnoser on (AK_BDN_MEMORY=1): one extra workload iteration per case after the actual stage, outside the job's clock
# grid:       codec: units incumbent-prod:default, core-ffi:retain, host-gen:retain (full build), core-ffi:no-unknown, host-gen:no-unknown (no-unknown build); encode-transport-hot and decode-read; 16 shapes (P7.1 decode only), Latin-1 and wide on P2.2, 7 U-* rows
#             rpc: transport armonik; stock h2: A, Bf (+ B a+read), Cf-retain (+ C-retain a+read), Ef-retain (+ E-retain a+read) on a+read, b (P2.2), c (P5.4), d (16 MiB) at k = 1 and 8; h2-batch: Cf-retain on c, d at k = 1, 8; pinned allocator: A, Cf-retain on c, d at k = 1
# dropped:    nothing of the core grid; calib not run; no plant controls; server warm-up 500 calls per direction (campaign 2000)
# allocator:  default (GLIBC_TUNABLES unset) for every process except the pinned-allocator subset (GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432)
# build: 24.1 s
# codec full: 272.9 s rc=0
# codec nounk: 201.5 s rc=0
# rpc server build: 0.2 s
# rpc server warm (500): 36.2 s
# rpc stock: 181.1 s rc=0
# rpc h2-batch: 50.0 s rc=0
# rpc stock-pinned: 39.7 s rc=0
# total: 808.0 s
```

## Codec suite

Units and the labels the rows carry (arm, unknown_mode, build): `core-ffi no-unknown no-unknown`; `core-ffi retain full`; `host-gen no-unknown no-unknown`; `host-gen retain full`; `incumbent-prod default full`. `incumbent-prod` runs in the full build only (section 4.0). Payloads: SHAPES.md; `/latin1`, `/wide` = content sets on P2.2; U-* = corpus rows (content `corpus`). encode = `encode-transport-hot` (end state ii, the form the arm hands its transport, one hot graph); decode = `decode-read` (decode, then read every field).

### encode (encode-transport-hot): process CPU per op, microseconds

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 1.308 [1.296-1.328] | 0.804 [0.787-1.443] * | 0.758 [0.754-1.854] * | 0.673 [0.663-0.728] | 0.731 [0.723-1.066] * |
| P1.2 | 383.3 [369.9-409.1] | 213.6 [207.7-262.4] * | 202.5 [197.9-326.4] * | 164.6 [161.8-172.1] | 159.3 [157.5-161.8] |
| P1.3 | 7.114 [7.041-15.50] * | 16.53 [16.14-31.94] * | 15.46 [15.37-29.47] * | 2.592 [2.562-4.272] * | 2.489 [2.481-4.823] * |
| P2.1 | 2.652 [2.635-2.704] | 1.440 [1.435-1.445] | 1.355 [1.342-1.385] | 1.010 [1.002-1.268] * | 0.987 [0.981-1.000] |
| P2.2 | 1843 [1827-1909] | 878.9 [868.6-1070] | 984.2 [917.9-1158] | 639.1 [616.0-672.2] | 676.7 [640.0-776.4] ^ |
| P2.2/latin1 | 2301 [2272-2347] | 1285 [1248-1990] * | 1175 [1159-1319] | 942.2 [908.3-1430] * ^ | 965.7 [937.4-997.6] |
| P2.2/wide | 2622 [2474-2739] | 1624 [1549-3426] * | 1560 [1526-1601] | 1355 [1309-1418] | 1245 [1229-1342] |
| P2.3 | 1121 [1077-1153] | 640.1 [622.2-803.7] * | 646.7 [640.1-1438] * | 419.0 [401.7-1039] * | 417.1 [411.4-428.8] |
| P2.4 | 1504 [1409-1763] | 1841 [1809-1925] | 1036 [904.7-1479] * | 562.8 [546.2-584.2] | 590.0 [577.6-627.8] |
| P2.5 | 48.48 [48.27-49.06] | 22.76 [22.70-23.79] | 22.20 [22.11-22.58] | 17.41 [17.21-18.70] | 17.10 [16.93-17.25] |
| P3.1 | 22.67 [22.35-22.77] | 13.19 [12.84-13.40] | 13.23 [12.72-27.08] * | 10.38 [10.28-10.81] | 10.13 [10.03-10.27] |
| P4.1 | 372.8 [363.0-639.2] * ^ | 114.7 [111.7-187.2] * | 114.5 [112.6-421.7] * | 87.78 [86.11-92.37] | 91.21 [89.33-101.5] |
| P5.1 | 0.161 [0.159-0.162] | 0.138 [0.135-0.277] * | 0.133 [0.132-0.138] | 0.104 [0.102-0.188] * | 0.115 [0.110-0.122] |
| P5.2 | 1.978 [1.765-3.276] * | 3.327 [3.321-3.403] | 3.483 [3.345-6.562] * | 3.602 [3.569-7.237] * | 3.317 [3.292-5.950] * |
| P5.3 | 70.60 [65.15-155.8] * | 130.3 [123.3-152.7] | 136.0 [130.3-233.9] * | 127.7 [126.6-134.3] | 152.7 [148.8-302.6] * |
| P5.4 | 284.7 [281.7-375.2] * | 633.2 [603.6-667.5] | 632.1 [606.9-862.8] * | 621.8 [592.7-1017] * | 570.0 [563.8-586.0] |
| P6.1 | 476.9 [458.7-490.7] | 145.0 [130.2-216.5] * | 129.7 [127.6-146.3] | 139.9 [137.0-140.5] | 138.1 [134.2-140.7] |
| U-deep-u-repeated | 2.712 [2.693-2.768] | 1.385 [1.340-2.700] * | 1.266 [1.238-1.971] * | 0.919 [0.915-0.981] | 0.910 [0.893-0.999] |
| U-nested-before | 0.776 [0.760-0.856] ^ | 0.319 [0.311-0.581] * | 0.282 [0.279-0.472] * | 0.226 [0.221-0.257] ^ | 0.221 [0.219-0.221] |
| U-oneof-u-repeated | 0.348 [0.340-0.424] ^ | 0.173 [0.173-0.181] | 0.165 [0.162-0.315] * | 0.104 [0.102-0.184] * | 0.105 [0.099-0.307] * |
| U-wire-DualResponse-left-as-wt5 | 0.370 [0.369-0.430] ^ | 0.267 [0.263-0.547] * | 0.256 [0.251-0.257] | 0.149 [0.147-0.160] | 0.145 [0.143-0.155] |
| U-wire-ListMetricsResponse-batches-as-wt0 | 4.258 [4.204-4.331] | 1.475 [1.454-1.482] | 1.510 [1.503-1.534] | 1.355 [1.316-1.554] | 1.315 [1.299-1.327] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 3.106 [3.065-3.439] ^ | 1.207 [1.187-2.306] * | 1.117 [1.115-2.066] * | 0.932 [0.924-0.949] | 0.921 [0.909-1.011] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 0.258 [0.254-0.265] | 0.145 [0.144-0.238] * | 0.139 [0.139-0.260] * | 0.110 [0.110-0.115] | 0.111 [0.108-0.115] |

### encode (encode-transport-hot): wall per op, microseconds

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 1.306 [1.293-1.326] | 0.808 [0.787-1.139] * | 0.763 [0.754-1.126] * | 0.688 [0.664-0.765] | 0.742 [0.725-0.846] ^ |
| P1.2 | 387.1 [369.2-516.1] * | 218.2 [209.5-283.2] * | 205.2 [199.7-230.5] | 164.4 [161.4-188.8] | 158.8 [156.7-161.2] |
| P1.3 | 7.500 [7.039-8.408] | 16.18 [16.12-18.46] | 15.44 [15.40-15.75] | 2.584 [2.559-2.600] | 2.490 [2.480-2.499] |
| P2.1 | 2.666 [2.630-2.787] | 1.442 [1.434-1.466] | 1.363 [1.341-1.509] | 1.022 [1.005-1.047] | 0.982 [0.978-1.034] |
| P2.2 | 1835 [1821-1913] | 873.7 [867.7-880.1] | 1017 [901.5-1209] * | 637.7 [614.3-795.3] * | 674.5 [638.1-1122] * |
| P2.2/latin1 | 2379 [2270-2482] | 1317 [1250-1387] | 1173 [1159-1381] | 966.7 [932.4-1063] | 964.0 [934.2-996.3] |
| P2.2/wide | 2614 [2469-2737] | 1635 [1548-2139] * | 1563 [1523-1617] | 1358 [1306-1686] * | 1242 [1226-1331] |
| P2.3 | 1144 [1075-1171] | 642.0 [621.7-712.0] | 646.9 [638.0-740.8] | 445.5 [401.4-708.5] * | 427.6 [410.0-436.0] |
| P2.4 | 1525 [1457-1817] | 1829 [1789-1905] | 991.7 [901.1-2075] * | 561.9 [544.6-643.4] ^ | 592.5 [575.3-630.5] |
| P2.5 | 48.32 [48.11-49.03] | 22.73 [22.68-25.73] ^ | 22.53 [22.15-22.95] | 17.83 [17.20-24.41] * ^ | 17.14 [16.96-17.40] |
| P3.1 | 22.54 [22.29-22.69] | 13.46 [12.86-14.92] | 14.10 [13.38-20.63] * | 10.74 [10.25-12.73] | 10.11 [10.03-10.17] |
| P4.1 | 375.6 [362.4-436.2] | 116.0 [111.5-137.8] | 115.1 [112.5-213.6] * | 89.19 [85.98-104.6] | 92.20 [89.11-103.0] |
| P5.1 | 0.160 [0.158-0.163] | 0.139 [0.135-0.152] | 0.136 [0.133-0.155] ^ | 0.107 [0.102-0.116] | 0.115 [0.109-0.166] * ^ |
| P5.2 | 1.789 [1.764-1.826] | 3.323 [3.316-3.352] | 3.427 [3.338-3.640] | 3.638 [3.580-3.735] | 3.319 [3.293-3.356] |
| P5.3 | 70.66 [65.08-80.46] | 148.7 [123.7-174.5] * | 131.6 [130.2-141.2] | 127.5 [126.3-134.3] | 153.1 [150.0-186.1] |
| P5.4 | 284.4 [281.1-293.6] | 635.3 [611.6-750.8] | 629.4 [618.5-744.3] | 642.1 [593.3-673.6] | 567.3 [561.2-580.6] |
| P6.1 | 508.7 [456.6-591.4] * | 136.0 [130.2-166.0] * ^ | 129.6 [127.5-136.6] | 140.4 [137.6-145.6] | 141.2 [133.4-143.9] |
| U-deep-u-repeated | 2.701 [2.686-2.897] | 1.359 [1.339-1.411] | 1.249 [1.243-1.857] * | 0.937 [0.907-0.985] | 0.980 [0.899-1.040] |
| U-nested-before | 0.787 [0.757-0.971] * | 0.321 [0.312-0.322] | 0.304 [0.282-0.331] | 0.227 [0.221-0.260] ^ | 0.224 [0.221-0.225] |
| U-oneof-u-repeated | 0.352 [0.339-0.423] ^ | 0.173 [0.173-0.181] | 0.168 [0.161-0.185] | 0.103 [0.103-0.111] | 0.101 [0.099-0.164] * |
| U-wire-DualResponse-left-as-wt5 | 0.369 [0.368-0.424] ^ | 0.279 [0.262-0.309] | 0.267 [0.251-0.279] | 0.153 [0.149-0.194] * | 0.150 [0.144-0.175] |
| U-wire-ListMetricsResponse-batches-as-wt0 | 4.233 [4.163-4.286] | 1.481 [1.468-1.648] ^ | 1.532 [1.513-1.599] | 1.393 [1.324-1.598] | 1.338 [1.301-1.463] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 3.114 [3.059-3.763] ^ | 1.209 [1.186-1.225] | 1.115 [1.115-1.121] | 0.948 [0.923-1.060] | 0.922 [0.907-1.105] ^ |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 0.263 [0.254-0.346] * | 0.149 [0.147-0.155] | 0.139 [0.138-0.140] | 0.112 [0.109-0.115] | 0.125 [0.111-0.161] * |

### encode (encode-transport-hot): allocated bytes per op, and Gen0/Gen1/Gen2 per 1,000 ops (MemoryDiagnoser iteration: ops)

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 0 B; 0/0/0 (7476 ops) | 0 B; 0/0/0 (50187 ops) | 0 B; 0/0/0 (53557 ops) | 0 B; 0/0/0 (59717 ops) | 0 B; 0/0/0 (54972 ops) |
| P1.2 | 0 B; 0/0/0 (26 ops) | 0 B; 0/0/0 (117 ops) | 0 B; 0/0/0 (201 ops) | 0 B; 0/0/0 (49 ops) | 0 B; 0/0/0 (51 ops) |
| P1.3 | 0 B; 0/0/0 (5553 ops) | 0 B; 0/0/0 (2486 ops) | 0 B; 0/0/0 (2601 ops) | 0 B; 0/0/0 (15349 ops) | 0 B; 0/0/0 (14668 ops) |
| P2.1 | 56 B; 0/0/0 (3092 ops) | 0 B; 0/0/0 (14626 ops) | 0 B; 0/0/0 (16194 ops) | 128 B; 0/0/0 (36609 ops) | 128 B; 0/0/0 (7809 ops) |
| P2.2 | 28,000 B; 0/0/0 (4 ops) | 0 B; 0/0/0 (33 ops) | 0 B; 0/0/0 (32 ops) | 64,000 B; 0/0/0 (13 ops) | 64,000 B; 0/0/0 (13 ops) |
| P2.2/latin1 | 28,000 B; 0/0/0 (6 ops) | 0 B; 0/0/0 (32 ops) | 0 B; 0/0/0 (26 ops) | 64,000 B; 0/0/0 (13 ops) | 64,000 B; 0/0/0 (12 ops) |
| P2.2/wide | 28,000 B; 0/0/0 (6 ops) | 0 B; 0/0/0 (24 ops) | 0 B; 0/0/0 (11 ops) | 64,000 B; 0/0/0 (11 ops) | 64,000 B; 0/0/0 (11 ops) |
| P2.3 | 7,000 B; 0/0/0 (11 ops) | 0 B; 0/0/0 (44 ops) | 0 B; 0/0/0 (50 ops) | 16,000 B; 0/0/0 (94 ops) | 16,000 B; 0/0/0 (24 ops) |
| P2.4 | 4,480 B; 0/0/0 (8 ops) | 0 B; 0/0/0 (16 ops) | 0 B; 0/0/0 (26 ops) | 10,240 B; 0/0/0 (17 ops) | 10,240 B; 0/0/0 (17 ops) |
| P2.5 | 1,120 B; 0/0/0 (223 ops) | 0 B; 0/0/0 (1704 ops) | 0 B; 0/0/0 (1803 ops) | 2,560 B; 0/0/0 (2344 ops) | 2,560 B; 0/0/0 (2343 ops) |
| P3.1 | 0 B; 0/0/0 (214 ops) | 0 B; 0/0/0 (1494 ops) | 0 B; 0/0/0 (1514 ops) | 0 B; 0/0/0 (3868 ops) | 0 B; 0/0/0 (3925 ops) |
| P4.1 | 11,200 B; 0/0/0 (34 ops) | 0 B; 0/0/0 (194 ops) | 0 B; 0/0/0 (348 ops) | 25,600 B; 0/0/0 (453 ops) | 25,600 B; 0/0/0 (87 ops) |
| P5.1 | 0 B; 0/0/0 (45204 ops) | 0 B; 0/0/0 (295251 ops) | 0 B; 0/0/0 (136584 ops) | 0 B; 0/0/0 (384208 ops) | 0 B; 0/0/0 (47646 ops) |
| P5.2 | 0 B; 0/0/0 (18932 ops) | 0 B; 0/0/0 (12069 ops) | 0 B; 0/0/0 (10689 ops) | 0 B; 0/0/0 (11079 ops) | 0 B; 0/0/0 (12158 ops) |
| P5.3 | 0 B; 0/0/0 (555 ops) | 0 B; 0/0/0 (305 ops) | 0 B; 0/0/0 (276 ops) | 0 B; 0/0/0 (302 ops) | 0 B; 0/0/0 (311 ops) |
| P5.4 | 0 B; 0/0/0 (125 ops) | 0 B; 0/0/0 (67 ops) | 0 B; 0/0/0 (64 ops) | 0 B; 0/0/0 (56 ops) | 0 B; 0/0/0 (67 ops) |
| P6.1 | 0 B; 0/0/0 (11 ops) | 0 B; 0/0/0 (305 ops) | 0 B; 0/0/0 (247 ops) | 0 B; 0/0/0 (44 ops) | 0 B; 0/0/0 (44 ops) |
| U-deep-u-repeated | 280 B; 0/0/0 (2277 ops) | 0 B; 0/0/0 (15804 ops) | 0 B; 0/0/0 (32012 ops) | 128 B; 0/0/0 (7713 ops) | 128 B; 0/0/0 (43058 ops) |
| U-nested-before | 168 B; 0/0/0 (7182 ops) | 0 B; 0/0/0 (128483 ops) | 0 B; 0/0/0 (145123 ops) | 0 B; 0/0/0 (151806 ops) | 0 B; 0/0/0 (142957 ops) |
| U-oneof-u-repeated | 168 B; 0/0/0 (17993 ops) | 0 B; 0/0/0 (222728 ops) | 0 B; 0/0/0 (237156 ops) | 0 B; 0/0/0 (393142 ops) | 0 B; 0/0/0 (388839 ops) |
| U-wire-DualResponse-left-as-wt5 | 112 B; 0/0/0 (20248 ops) | 0 B; 0/0/0 (73512 ops) | 0 B; 0/0/0 (154976 ops) | 0 B; 0/0/0 (257383 ops) | 0 B; 0/0/0 (278920 ops) |
| U-wire-ListMetricsResponse-batches-as-wt0 | 112 B; 0/0/0 (962 ops) | 0 B; 0/0/0 (26154 ops) | 0 B; 0/0/0 (26708 ops) | 0 B; 0/0/0 (28287 ops) | 0 B; 0/0/0 (30637 ops) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 224 B; 0/0/0 (2188 ops) | 0 B; 0/0/0 (32996 ops) | 0 B; 0/0/0 (35084 ops) | 256 B; 0/0/0 (40188 ops) | 256 B; 0/0/0 (39729 ops) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 112 B; 0/0/0 (26246 ops) | 0 B; 0/0/0 (279894 ops) | 0 B; 0/0/0 (288320 ops) | 0 B; 0/0/0 (53872 ops) | 0 B; 0/0/0 (52400 ops) |

### encode (encode-transport-hot): core-ffi crossings per op, fwd/rev/grow/reset (committed count files)

| payload | core-ffi retain (gen/counts.txt) | core-ffi no-unknown (gen/counts-nounk.txt) |
|---|---:|---:|
| P1.1 | 4/1/0/0 | 4/1/0/0 |
| P1.2 | 4/1/0/0 | 4/1/0/0 |
| P1.3 | 4/1/0/0 | 4/1/0/0 |
| P2.1 | 9/6/0/0 | 9/6/0/0 |
| P2.2 | 2504/2501/0/0 | 2504/2501/0/0 |
| P2.2/latin1 | 2504/2501/0/0 | 2504/2501/0/0 |
| P2.2/wide | 2504/2501/0/0 | 2504/2501/0/0 |
| P2.3 | 629/626/0/0 | 629/626/0/0 |
| P2.4 | 404/401/0/0 | 404/401/0/0 |
| P2.5 | 104/101/0/0 | 104/101/0/0 |
| P3.1 | 4/1/0/0 | 4/1/0/0 |
| P4.1 | 204/201/0/0 | 204/201/0/0 |
| P5.1 | 3/0/0/0 | 3/0/0/0 |
| P5.2 | 3/0/0/0 | 3/0/0/0 |
| P5.3 | 3/0/0/0 | 3/0/0/0 |
| P5.4 | 3/0/0/0 | 3/0/0/0 |
| P6.1 | 1004/1001/0/0 | 1004/1001/0/0 |
| U-deep-u-repeated | 9/6/0/0 | 9/6/0/0 |
| U-nested-before | 4/1/0/0 | 4/1/0/0 |
| U-oneof-u-repeated | 4/1/0/0 | 4/1/0/0 |
| U-wire-DualResponse-left-as-wt5 | 5/2/0/0 | 5/2/0/0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | 14/11/0/0 | 14/11/0/0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 6/3/0/0 | 6/3/0/0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 3/0/0/0 | 3/0/0/0 |

### decode-read: process CPU per op, microseconds

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 1.855 [1.784-2.020] | 1.861 [1.830-1.909] | 1.813 [1.802-1.906] | 1.526 [1.466-1.653] | 1.503 [1.474-1.676] |
| P1.2 | 539.6 [498.7-662.2] * ^ | 508.2 [469.1-597.1] * | 485.7 [479.1-493.9] | 420.0 [410.8-443.3] | 447.8 [429.3-471.0] |
| P1.3 | 18.84 [18.06-43.26] * | 23.68 [22.67-37.90] * | 23.45 [22.46-45.70] * | 14.49 [13.90-19.15] * | 13.83 [13.33-14.25] |
| P2.1 | 3.341 [3.193-3.473] | 3.869 [3.781-6.902] * | 4.194 [3.934-13.09] * | 2.622 [2.555-3.331] * ^ | 2.690 [2.548-2.800] |
| P2.2 | 1957 [1916-2306] ^ | 2656 [2443-2896] | 2612 [2497-2692] | 1944 [1838-2081] | 2070 [1766-2137] |
| P2.2/latin1 | 2562 [2533-3121] ^ | 3135 [3100-3293] | 3361 [3090-3433] | 2539 [2352-2682] | 2697 [2481-2779] |
| P2.2/wide | 3201 [3046-3443] | 3663 [3568-3972] | 4038 [3888-4324] | 2762 [2718-2995] | 2883 [2806-3347] |
| P2.3 | 1434 [1350-1589] ^ | 1633 [1585-1662] | 1741 [1662-2328] * | 1417 [1358-1467] | 1320 [1295-1536] ^ |
| P2.4 | 1871 [1831-3737] * ^ | 2533 [2348-2949] | 2327 [2259-2583] ^ | 1861 [1743-2100] ^ | 1866 [1746-2178] ^ |
| P2.5 | 62.86 [62.06-67.72] | 78.76 [71.11-129.5] * | 69.50 [68.38-126.7] * | 51.50 [50.54-52.83] | 49.81 [49.04-50.83] |
| P3.1 | 35.41 [33.70-44.72] * ^ | 35.53 [34.66-37.42] | 35.32 [34.45-36.94] | 26.56 [25.81-33.42] * ^ | 28.00 [27.05-33.69] ^ |
| P4.1 | 332.3 [324.5-396.6] ^ | 402.1 [387.5-435.3] | 390.1 [386.9-404.8] | 287.0 [271.9-367.7] * ^ | 304.9 [291.6-422.2] * ^ |
| P5.1 | 0.222 [0.215-0.234] | 0.277 [0.268-0.331] ^ | 0.265 [0.258-0.316] ^ | 0.156 [0.151-0.167] | 0.151 [0.146-0.163] |
| P5.2 | 9.387 [8.564-16.08] * | 10.38 [9.795-15.33] * | 11.52 [9.877-17.21] * | 10.86 [10.40-21.11] * | 9.386 [8.596-10.77] ^ |
| P5.3 | 690.0 [644.0-1033] * | 785.5 [736.0-827.8] | 828.4 [678.3-1436] * ^ | 732.5 [676.4-891.7] * | 747.6 [688.5-878.7] * |
| P5.4 | 1751 [1683-1826] | 1047 [983.3-1131] | 1815 [1698-2234] * | 2093 [2063-3168] * | 1931 [1783-2292] * |
| P6.1 | 351.9 [342.9-396.4] ^ | 343.5 [334.7-366.2] | 349.8 [342.2-362.7] | 335.6 [320.9-419.9] * ^ | 388.8 [380.4-437.1] ^ |
| P7.1 | 0.661 [0.647-0.687] | 0.657 [0.645-0.767] ^ | 0.663 [0.656-0.668] | 0.448 [0.421-0.460] | 0.407 [0.398-0.416] |
| U-deep-u-repeated | 3.182 [3.124-3.328] | 3.853 [3.783-3.998] | 3.647 [3.540-6.756] * | 2.494 [2.366-2.573] | 2.402 [2.315-2.651] |
| U-nested-before | 1.365 [1.339-1.453] | 0.962 [0.949-0.975] | 0.680 [0.666-0.693] | 0.674 [0.648-0.752] ^ | 0.482 [0.466-0.491] |
| U-oneof-u-repeated | 0.492 [0.464-0.510] | 0.510 [0.499-0.531] | 0.336 [0.326-0.397] | 0.240 [0.234-0.266] | 0.198 [0.194-0.209] |
| U-wire-DualResponse-left-as-wt5 | 0.614 [0.606-0.647] | 0.721 [0.702-0.749] | 0.518 [0.507-0.567] | 0.334 [0.328-0.342] | 0.322 [0.318-0.349] |
| U-wire-ListMetricsResponse-batches-as-wt0 | 3.868 [3.756-3.907] | 4.043 [3.951-7.561] * | 3.651 [3.538-3.774] | 3.380 [3.267-3.515] | 3.691 [3.528-3.992] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 3.141 [3.116-3.967] * ^ | 3.672 [3.611-3.703] | 3.347 [3.312-3.465] | 2.563 [2.448-2.758] | 2.582 [2.513-2.620] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 0.349 [0.339-0.363] | 0.428 [0.423-0.446] | 0.267 [0.262-0.312] ^ | 0.183 [0.174-0.459] * | 0.144 [0.142-0.151] |

### decode-read: wall per op, microseconds

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 1.892 [1.779-2.503] * | 1.860 [1.828-2.128] ^ | 1.811 [1.807-1.946] | 1.547 [1.463-1.667] | 1.500 [1.472-1.678] |
| P1.2 | 589.6 [497.8-723.3] * ^ | 560.8 [485.6-600.7] | 485.8 [479.5-526.9] | 419.4 [410.5-442.9] | 447.1 [428.7-470.6] |
| P1.3 | 19.17 [18.18-24.96] * | 23.65 [22.96-24.25] | 22.59 [22.23-23.40] | 14.48 [13.90-14.75] | 13.80 [13.31-14.23] |
| P2.1 | 3.516 [3.196-4.058] ^ | 3.912 [3.781-4.062] | 5.307 [3.972-6.609] * | 2.621 [2.556-3.417] * ^ | 2.662 [2.543-3.020] |
| P2.2 | 1978 [1921-2293] ^ | 2649 [2450-2891] | 2624 [2511-2824] | 2016 [1870-2075] | 2067 [1764-2153] |
| P2.2/latin1 | 2558 [2539-3098] ^ | 3139 [3110-3282] | 3312 [3088-3432] | 2547 [2359-2676] | 2718 [2517-2778] |
| P2.2/wide | 3233 [3036-3546] | 3737 [3566-4021] | 4033 [3883-5692] * ^ | 2757 [2712-2990] | 2957 [2876-3470] |
| P2.3 | 1432 [1367-1586] ^ | 1654 [1583-1727] | 1741 [1682-1783] | 1458 [1402-1670] | 1317 [1300-1539] ^ |
| P2.4 | 1930 [1845-2293] ^ | 2539 [2361-2773] | 2337 [2256-2619] ^ | 1995 [1777-2189] | 1869 [1751-2176] ^ |
| P2.5 | 68.93 [63.13-85.56] * | 75.69 [71.05-94.52] * | 69.72 [68.20-70.53] | 51.58 [51.35-53.33] | 49.50 [48.79-62.10] * ^ |
| P3.1 | 38.37 [34.34-79.50] * | 35.48 [34.62-37.37] | 35.94 [34.42-36.90] | 26.84 [25.76-33.59] * ^ | 29.39 [27.08-34.70] * |
| P4.1 | 331.6 [323.9-395.8] ^ | 425.5 [389.0-608.9] * | 395.1 [387.4-436.1] | 294.3 [275.8-369.6] * ^ | 330.1 [291.0-517.4] * ^ |
| P5.1 | 0.237 [0.214-0.259] | 0.285 [0.267-0.335] ^ | 0.264 [0.257-0.319] ^ | 0.156 [0.151-0.195] * | 0.152 [0.147-0.167] |
| P5.2 | 9.379 [8.558-10.64] | 10.48 [9.849-11.35] | 10.93 [9.082-11.88] * | 11.11 [10.50-13.70] * | 9.768 [8.951-10.80] |
| P5.3 | 692.1 [597.9-757.1] | 770.4 [664.8-820.9] | 873.9 [805.2-1532] * ^ | 751.3 [724.7-876.4] ^ | 808.3 [668.7-867.5] |
| P5.4 | 1793 [1722-2100] | 1051 [981.9-1195] | 1871 [1782-2278] * ^ | 2091 [2068-2487] | 2002 [1834-2105] |
| P6.1 | 349.3 [340.2-400.1] ^ | 348.2 [336.4-482.9] * | 348.9 [341.8-362.2] | 338.0 [320.1-431.8] * ^ | 384.2 [380.1-438.1] ^ |
| P7.1 | 0.679 [0.646-0.801] | 0.663 [0.645-0.788] ^ | 0.662 [0.655-0.667] | 0.448 [0.420-0.572] * | 0.416 [0.404-0.428] |
| U-deep-u-repeated | 3.176 [3.117-3.321] | 3.869 [3.780-4.284] | 3.646 [3.539-3.696] | 2.490 [2.359-2.555] | 2.397 [2.310-2.646] |
| U-nested-before | 1.362 [1.332-1.445] | 0.962 [0.947-0.982] | 0.682 [0.663-0.694] | 0.683 [0.647-0.946] * | 0.488 [0.469-0.495] |
| U-oneof-u-repeated | 0.501 [0.462-0.750] * | 0.509 [0.499-0.528] | 0.338 [0.326-0.476] * | 0.239 [0.234-0.266] | 0.198 [0.194-0.349] * ^ |
| U-wire-DualResponse-left-as-wt5 | 0.611 [0.602-0.641] | 0.723 [0.701-0.755] | 0.523 [0.506-0.587] | 0.334 [0.327-0.342] | 0.322 [0.318-0.346] |
| U-wire-ListMetricsResponse-batches-as-wt0 | 3.880 [3.745-3.909] | 4.657 [3.999-5.400] * | 3.660 [3.533-3.762] | 3.447 [3.289-5.359] * ^ | 3.734 [3.526-4.018] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 3.142 [3.110-3.961] * ^ | 3.776 [3.668-3.871] | 3.339 [3.307-3.486] | 2.556 [2.441-2.744] | 2.579 [2.507-2.603] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 0.361 [0.339-0.380] | 0.443 [0.429-0.458] | 0.269 [0.261-0.311] ^ | 0.184 [0.172-0.636] * | 0.143 [0.141-0.151] |

### decode-read: allocated bytes per op, and Gen0/Gen1/Gen2 per 1,000 ops (MemoryDiagnoser iteration: ops)

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 2,888 B; 0/0/0 (3974 ops) | 2,728 B; 0.12/0/0 (8459 ops) | 2,728 B; 0.14/0/0 (21789 ops) | 2,728 B; 0/0/0 (5900 ops) | 2,728 B; 0.13/0/0 (7456 ops) |
| P1.2 | 697,928 B; 0/0/0 (19 ops) | 665,984 B; 28.17/14.08/0 (71 ops) | 665,984 B; 33.9/16.95/0 (59 ops) | 665,984 B; 27.78/0/0 (36 ops) | 665,984 B; 37.04/0/0 (27 ops) |
| P1.3 | 39,568 B; 2.27/0/0 (1321 ops) | 39,624 B; 2.28/0/0 (1753 ops) | 39,624 B; 1.73/0/0 (1731 ops) | 39,624 B; 2.25/0/0 (2672 ops) | 39,624 B; 1.3/0/0 (768 ops) |
| P2.1 | 4,904 B; 0/0/0 (2541 ops) | 4,544 B; 0.16/0/0 (6248 ops) | 4,544 B; 0.16/0/0 (6065 ops) | 4,544 B; 0/0/0 (3521 ops) | 4,544 B; 0/0/0 (3404 ops) |
| P2.2 | 2,344,528 B; 0/0/0 (6 ops) | 2,180,584 B; 90.91/0/0 (11 ops) | 2,180,584 B; 83.33/0/0 (12 ops) | 2,180,584 B; 111.11/0/0 (9 ops) | 2,180,584 B; 111.11/0/0 (9 ops) |
| P2.2/latin1 | 2,344,528 B; 0/0/0 (4 ops) | 2,180,584 B; 100/0/0 (10 ops) | 2,180,584 B; 100/0/0 (10 ops) | 2,180,584 B; 125/0/0 (8 ops) | 2,180,584 B; 125/0/0 (8 ops) |
| P2.2/wide | 2,344,528 B; 0/0/0 (4 ops) | 2,180,584 B; 0/0/0 (7 ops) | 2,180,584 B; 125/0/0 (8 ops) | 2,180,584 B; 0/0/0 (7 ops) | 2,180,584 B; 0/0/0 (7 ops) |
| P2.3 | 2,098,104 B; 111.11/0/0 (9 ops) | 2,101,160 B; 62.5/0/0 (16 ops) | 2,101,160 B; 111.11/55.56/0 (18 ops) | 2,101,160 B; 83.33/0/0 (12 ops) | 2,101,160 B; 83.33/0/0 (12 ops) |
| P2.4 | 3,287,832 B; 142.86/0/0 (7 ops) | 3,275,728 B; 187.5/125/0 (16 ops) | 3,275,728 B; 181.82/90.91/0 (11 ops) | 3,275,728 B; 125/0/0 (8 ops) | 3,275,728 B; 125/0/0 (8 ops) |
| P2.5 | 87,976 B; 0/0/0 (142 ops) | 81,472 B; 3.61/0/0 (277 ops) | 81,472 B; 2.79/0/0 (358 ops) | 81,472 B; 4.55/0/0 (220 ops) | 81,472 B; 0/0/0 (174 ops) |
| P3.1 | 49,664 B; 0/0/0 (308 ops) | 53,880 B; 2.51/0/0 (399 ops) | 53,880 B; 1.92/0/0 (521 ops) | 53,880 B; 0/0/0 (284 ops) | 53,880 B; 0/0/0 (306 ops) |
| P4.1 | 403,704 B; 0/0/0 (34 ops) | 363,760 B; 16.95/0/0 (59 ops) | 363,760 B; 16.95/0/0 (59 ops) | 363,760 B; 0/0/0 (34 ops) | 363,760 B; 0/0/0 (36 ops) |
| P5.1 | 368 B; 0/0/0 (38898 ops) | 336 B; 0/0/0 (45068 ops) | 336 B; 0/0/0 (43643 ops) | 336 B; 0.02/0/0 (268368 ops) | 336 B; 0.01/0/0 (73200 ops) |
| P5.2 | 65,864 B; 3.54/0.64/0 (3103 ops) | 65,832 B; 3.76/0.38/0 (2659 ops) | 65,832 B; 3.7/0.53/0 (3783 ops) | 65,832 B; 3.8/0.58/0 (3424 ops) | 65,832 B; 3.59/0.48/0 (4176 ops) |
| P5.3 | 1,049,067 B; 74.07/74.07/74.07 (54 ops) | 1,049,204 B; 103.45/103.45/103.45 (58 ops) | 1,049,030 B; 68.97/68.97/68.97 (58 ops) | 1,049,424 B; 93.75/93.75/93.75 (64 ops) | 1,049,093 B; 75/75/75 (80 ops) |
| P5.4 | 4,194,657 B; 76.92/76.92/76.92 (26 ops) | 4,194,600 B; 0/0/0 (18 ops) | 4,194,628 B; 86.96/86.96/86.96 (23 ops) | 4,194,628 B; 86.96/86.96/86.96 (23 ops) | 4,194,625 B; 76.92/76.92/76.92 (26 ops) |
| P6.1 | 356,240 B; 0/0/0 (15 ops) | 466,696 B; 17.86/0/0 (56 ops) | 466,696 B; 22.73/0/0 (44 ops) | 466,696 B; 0/0/0 (25 ops) | 466,696 B; 0/0/0 (23 ops) |
| P7.1 | 776 B; 0.03/0/0 (60395 ops) | 712 B; 0/0/0 (23728 ops) | 712 B; 0/0/0 (24195 ops) | 712 B; 0/0/0 (21456 ops) | 712 B; 0/0/0 (20032 ops) |
| U-deep-u-repeated | 5,000 B; 0/0/0 (2592 ops) | 4,152 B; 0.18/0/0 (5634 ops) | 4,096 B; 0.18/0/0 (5470 ops) | 4,192 B; 0/0/0 (3699 ops) | 4,096 B; 0/0/0 (3804 ops) |
| U-nested-before | 2,688 B; 0.15/0/0 (6747 ops) | 904 B; 0/0/0 (17326 ops) | 784 B; 0.04/0/0 (23268 ops) | 1,384 B; 0.07/0/0 (13789 ops) | 784 B; 0.04/0/0 (85213 ops) |
| U-oneof-u-repeated | 904 B; 0.04/0/0 (25349 ops) | 400 B; 0/0/0 (26921 ops) | 352 B; 0/0/0 (36111 ops) | 440 B; 0.02/0/0 (40116 ops) | 352 B; 0/0/0 (43860 ops) |
| U-wire-DualResponse-left-as-wt5 | 984 B; 0/0/0 (14040 ops) | 568 B; 0/0/0 (22307 ops) | 536 B; 0/0/0 (28212 ops) | 568 B; 0.03/0/0 (117975 ops) | 536 B; 0/0/0 (27040 ops) |
| U-wire-ListMetricsResponse-batches-as-wt0 | 4,072 B; 0/0/0 (1311 ops) | 4,776 B; 0/0/0 (3400 ops) | 4,744 B; 0.27/0/0 (3761 ops) | 4,776 B; 0/0/0 (1998 ops) | 4,744 B; 0.2/0/0 (10185 ops) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 4,504 B; 0/0/0 (2942 ops) | 3,720 B; 0.19/0/0 (5295 ops) | 3,688 B; 0.16/0/0 (6136 ops) | 3,720 B; 0/0/0 (3775 ops) | 3,688 B; 0/0/0 (3823 ops) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 728 B; 0.03/0/0 (28705 ops) | 344 B; 0/0/0 (31212 ops) | 312 B; 0/0/0 (45956 ops) | 344 B; 0.02/0/0 (65424 ops) | 312 B; 0.01/0/0 (67520 ops) |

### decode-read: core-ffi crossings per op, fwd/rev/grow/reset (committed count files)

| payload | core-ffi retain (gen/counts.txt) | core-ffi no-unknown (gen/counts-nounk.txt) |
|---|---:|---:|
| P1.1 | 4/2/0/1 | 3/2/0/0 |
| P1.2 | 4/8/0/1 | 3/5/0/0 |
| P1.3 | 4/3/0/1 | 3/3/0/0 |
| P2.1 | 4/8/0/1 | 3/8/0/0 |
| P2.2 | 4/3501/0/1 | 3/3501/0/0 |
| P2.2/latin1 | 4/3501/0/1 | 3/3501/0/0 |
| P2.2/wide | 4/3501/0/1 | 3/3501/0/0 |
| P2.3 | 4/876/0/1 | 3/876/0/0 |
| P2.4 | 4/561/0/1 | 3/561/0/0 |
| P2.5 | 4/141/0/1 | 3/141/0/0 |
| P3.1 | 4/2/0/1 | 3/2/0/0 |
| P4.1 | 4/601/0/1 | 3/601/0/0 |
| P5.1 | 4/1/0/1 | 3/1/0/0 |
| P5.2 | 4/1/0/1 | 3/1/0/0 |
| P5.3 | 4/1/0/1 | 3/1/0/0 |
| P5.4 | 4/1/0/1 | 3/1/0/0 |
| P6.1 | 4/1401/0/1 | 3/1401/0/0 |
| P7.1 | 4/3/0/1 | 3/3/0/0 |
| U-deep-u-repeated | 4/8/1/1 | 3/8/0/0 |
| U-nested-before | 4/2/2/1 | 3/2/0/0 |
| U-oneof-u-repeated | 4/2/1/1 | 3/2/0/0 |
| U-wire-DualResponse-left-as-wt5 | 4/3/1/1 | 3/3/0/0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | 4/15/1/1 | 3/15/0/0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 4/7/1/1 | 3/7/0/0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 4/1/1/1 | 3/1/0/0 |

## RPC grid

Per call: the client process's CPU (`cpu_ns` = perf task-clock of the whole process, CAMPAIGN req 21 as amended; `proc_cpu_ns` = CLOCK_PROCESS_CPUTIME_ID beside it) or wall of one BDN iteration divided by its calls, in **microseconds**; k = calls in flight (one invocation = k calls). Transport `armonik` (cell A: packages/csharp GrpcChannelFactory; the core cells: the core's shipped client configuration), TCP 127.0.0.1, the one Rust server. Direction a+read has an empty request, so the framed cells' a+read row is the reference cell's (B, C-retain, E-retain), run in the same BDN run under its own name, as the runner does. Crossings: per call from `gen/rpc-counts.txt` (A has none: Grpc.Net). Rows: stock h2 and the default allocator unless labelled.

### direction a+read (P2.2)

| cell | h2 / alloc | k | task-clock CPU / call | process CPU / call | wall / call | alloc B / call; Gen0/1/2 per 1k calls | minflt / call | crossings fwd/rev/grow/reset | client softirq ticks |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A | stock / default | 1 | 11631 [6563-13204] * ^ | 11396 [6269-12553] * | 7246 [6054-11018] * ^ | 2,355,392; 100/0/0 (10) | 5.2 | 0/0/0/0 | 2 |
| A | stock / default | 8 | 7085 [5420-9693] * | 6611 [5391-9417] * | 4321 [3734-6002] * | 2,358,445; 125/93.8/0 (32) | 21.3 | 0/0/0/0 | 0 |
| B | stock / default | 1 | 3191 [3032-6269] * | 3117 [2982-6072] * | 3381 [3189-3810] | 2,344,552; 111.1/0/0 (9) | 0.0 | 2/0/0/0 | 1 |
| B | stock / default | 8 | 4669 [4160-7233] * | 4222 [3935-7237] * | 3148 [2757-4176] * | 2,344,531; 125/93.8/0 (32) | 3.4 | 2/0/0/0 | 1 |
| C-retain | stock / default | 1 | 4139 [3783-6273] * | 3946 [3668-6043] * | 4355 [4023-5115] * | 2,180,608; 66.7/0/0 (15) | 0.8 | 6/3501/0/1 | 1 |
| C-retain | stock / default | 8 | 5382 [4791-7081] * | 5153 [4666-7080] * | 3383 [2973-4364] * | 2,180,587; 125/93.8/0 (32) | 4.7 | 6/3501/0/1 | 0 |
| E-retain | stock / default | 1 | 3261 [3166-7945] * | 3247 [3171-7784] * | 3590 [3326-4863] * | 2,180,608; 90.9/0/0 (11) | 0.0 | 2/0/0/0 | 3 |
| E-retain | stock / default | 8 | 3728 [3198-5427] * | 3663 [3172-5266] * | 2542 [2064-3271] * | 2,180,587; 125/93.8/0 (32) | 3.1 | 2/0/0/0 | 1 |

### direction b (P2.2)

| cell | h2 / alloc | k | task-clock CPU / call | process CPU / call | wall / call | alloc B / call; Gen0/1/2 per 1k calls | minflt / call | crossings fwd/rev/grow/reset | client softirq ticks |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A | stock / default | 1 | 16156 [13936-26018] * | 15012 [13526-23792] * | 11945 [8584-15020] * ^ | 581,800; 0/0/0 (8) | 18.1 | 0/0/0/0 | 0 |
| A | stock / default | 8 | 6464 [5800-9161] * | 6284 [5482-9015] * | 4046 [3595-5144] * | 581,380; 0/0/0 (32) | 48.2 | 0/0/0/0 | 4 |
| Bf | stock / default | 1 | 3517 [3158-6259] * | 3319 [3128-6002] * | 6165 [5564-7392] * | 28,056; 0/0/0 (9) | 0.1 | 2/0/0/0 | 1 |
| Bf | stock / default | 8 | 3369 [2892-4694] * | 3241 [2895-4384] * | 3023 [2609-3378] * | 28,035; 0/0/0 (32) | 0.4 | 2/0/0/0 | 2 |
| Cf-retain | stock / default | 1 | 2757 [2457-5084] * | 2608 [2327-4896] * | 4868 [4568-5456] | 72; 0/0/0 (13) | 0.6 | 2505/2501/0/0 | 2 |
| Cf-retain | stock / default | 8 | 2547 [2268-3794] * | 2513 [2213-3642] * | 2684 [2479-3042] | 51; 0/0/0 (32) | 0.0 | 2505/2501/0/0 | 1 |
| Ef-retain | stock / default | 1 | 1578 [1543-4226] * | 1564 [1528-4095] * | 3713 [3510-4144] ^ | 64,072; 0/0/0 (14) | 0.1 | 2/0/0/0 | 1 |
| Ef-retain | stock / default | 8 | 1575 [1500-2281] * | 1548 [1495-2293] * | 2140 [2047-2310] | 64,051; 0/0/0 (32) | 0.9 | 2/0/0/0 | 2 |

### direction c (P5.4)

| cell | h2 / alloc | k | task-clock CPU / call | process CPU / call | wall / call | alloc B / call; Gen0/1/2 per 1k calls | minflt / call | crossings fwd/rev/grow/reset | client softirq ticks |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A | stock / default | 1 | 13821 [11629-16306] * | 13596 [11922-16832] * | 10334 [9618-12365] * | 4,241,944; 0/0/0 (8) | 887.1 | 0/0/0/0 | 3 |
| A | stock / default | 8 | 9688 [7962-11459] * | 9568 [7732-11397] * | 6328 [5978-8948] * | 4,242,237; 0/0/0 (32) | 191.2 | 0/0/0/0 | 16 |
| A | stock / **pinned** | 1 | 16496 [11573-20207] * | 14667 [10958-20764] * | 12072 [10482-16869] * | 4,242,097; 0/0/0 (8) | 383.6 | 0/0/0/0 | 11 |
| Bf | stock / default | 1 | 5016 [4453-7529] * | 4843 [4433-5778] * | 7680 [6913-9475] * | 136; 0/0/0 (12) | 0.1 | 2/0/0/0 | 7 |
| Bf | stock / default | 8 | 5077 [4607-5850] | 4848 [4372-5705] * | 5859 [5163-6207] | 115; 0/0/0 (32) | 0.2 | 2/0/0/0 | 18 |
| Cf-retain | **h2-batch** / default | 1 | 4367 [4073-7299] * | 4145 [3950-7001] * | 7603 [6397-8842] * | 72; 0/0/0 (17) | 0.0 | 4/0/0/0 | 11 |
| Cf-retain | **h2-batch** / default | 8 | 4075 [3781-4975] * | 3976 [3621-4700] * | 5637 [5044-5897] | 51; 0/0/0 (32) | 0.1 | 4/0/0/0 | 17 |
| Cf-retain | stock / default | 1 | 4333 [4080-5351] * | 4190 [3947-5267] * | 7211 [6366-7613] | 72; 0/0/0 (13) | 0.3 | 4/0/0/0 | 10 |
| Cf-retain | stock / default | 8 | 4763 [4318-7558] * | 4387 [4227-5820] * | 5891 [5324-7900] * | 51; 0/0/0 (32) | 0.4 | 4/0/0/0 | 25 |
| Cf-retain | stock / **pinned** | 1 | 4643 [3833-7253] * | 4358 [3791-5247] * | 7303 [6451-11927] * | 72; 0/0/0 (13) | 0.0 | 4/0/0/0 | 9 |
| Ef-retain | stock / default | 1 | 5168 [4143-6777] * | 4913 [4114-6397] * | 6605 [5484-7198] * | 136; 0/0/0 (10) | 0.5 | 2/0/0/0 | 7 |
| Ef-retain | stock / default | 8 | 5455 [4787-5666] | 5381 [4437-5591] | 5943 [5112-6221] | 115; 0/0/0 (32) | 0.5 | 2/0/0/0 | 20 |

### direction d (stream-16MiB)

| cell | h2 / alloc | k | task-clock CPU / call | process CPU / call | wall / call | alloc B / call; Gen0/1/2 per 1k calls | minflt / call | crossings fwd/rev/grow/reset | client softirq ticks |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A | stock / default | 1 | 36123 [25898-38806] * | 35513 [25039-37953] * | 22776 [19790-25203] | 16,956,798; 0/0/0 (4) | 1.5 | 0/0/0/0 | 10 |
| A | stock / default | 8 | 32993 [30896-34816] | 32648 [31278-34400] | 22745 [20406-24499] | 16,960,638; 62.5/62.5/62.5 (32) | 568.5 | 0/0/0/0 | 61 |
| A | stock / **pinned** | 1 | 39423 [33000-68309] * | 38572 [31125-65054] * | 27565 [25112-34649] * | 16,956,420; 0/0/0 (4) | 2294.5 | 0/0/0/0 | 10 |
| Bf | stock / default | 1 | 16696 [14728-22992] * | 16245 [14197-21596] * | 14760 [13735-17455] * | 848; 0/0/0 (7) | 0.5 | 12/0/0/0 | 14 |
| Bf | stock / default | 8 | 19148 [18359-20940] | 18274 [17394-19714] | 18196 [17855-20348] | 827; 0/0/0 (32) | 0.3 | 12/0/0/0 | 60 |
| Cf-retain | **h2-batch** / default | 1 | 16863 [15173-22084] * | 16643 [14915-21462] * | 15581 [14211-18522] * | 336; 0/0/0 (6) | 0.0 | 28/0/0/0 | 13 |
| Cf-retain | **h2-batch** / default | 8 | 16876 [15690-19258] | 15364 [14949-18463] | 18348 [16366-20361] | 315; 0/0/0 (32) | 0.2 | 28/0/0/0 | 62 |
| Cf-retain | stock / default | 1 | 14145 [13835-19800] * | 13649 [13236-18779] * | 14569 [13336-16500] | 336; 0/0/0 (7) | 0.5 | 28/0/0/0 | 14 |
| Cf-retain | stock / default | 8 | 15743 [15157-17068] | 14844 [14258-16786] | 16971 [16064-17631] | 315; 0/0/0 (32) | 0.2 | 28/0/0/0 | 60 |
| Cf-retain | stock / **pinned** | 1 | 17414 [15623-26810] * | 16486 [14322-25109] * | 17477 [16022-21694] * | 336; 0/0/0 (4) | 0.0 | 28/0/0/0 | 7 |
| Ef-retain | stock / default | 1 | 17272 [15754-25524] * | 16178 [15359-24304] * | 15205 [14170-17891] | 848; 0/0/0 (7) | 0.1 | 12/0/0/0 | 19 |
| Ef-retain | stock / default | 8 | 18344 [17633-20025] | 17751 [16967-18720] | 17509 [16529-19458] ^ | 827; 0/0/0 (32) | 0.2 | 12/0/0/0 | 63 |

