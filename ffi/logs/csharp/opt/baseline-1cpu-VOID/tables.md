# C# baseline (optimisation pass): core grid, absolute times per case

**VOID for managed arms: client on 1 CPU (CLIENT=1), tier-0 suspected (.NET 8 multiplies the tier-up call-counting delay by TC_DelaySingleProcMultiplier=10 when the process is affinitized to one CPU: ~1 s, restarted at each new tier-0 JIT, longer than each BDN child's measurement); kept as a record, superseded by ../baseline/ (2-CPU client). See JOURNAL 73.**

CONTAINER INSTRUMENTATION, not a result (README 1.1); NOT GATED (gen/gate.sh not run; every codec process ran Cases.Verify, byte identity of every encode arm and variant and every U-* row, before timing; every RPC call checked). One `gen/opt_bench.sh` run, one launch, short BenchmarkDotNet settings (header below). No ratio is formed here.

Notation: each entry is the **median over the BDN actual iterations (rounds) of one case** with **[min-max]** across them; one round = one BDN iteration; per-op value = that iteration's CPU (or wall) / its operations. ` *` = (max - min) > 25 % of the median; ` ^` = round 1 is the slowest and > 10 % above the median (a warm-up tail). Every case ran in its own child process (BDN default toolchain). Allocation and GC: BenchmarkDotNet MemoryDiagnoser, one extra workload iteration per case after the actual stage, GC.GetTotalAllocatedBytes(precise) over every thread / its operations; Gen0/1/2 = collections during that iteration per 1,000 operations (with the operation count beside it). Crossing counts fwd/rev/grow/reset per operation are from the committed count files (`gen/counts.txt`, `gen/counts-nounk.txt`, `gen/rpc-counts.txt`, gated; none measured in this run).

```
# VOID for managed arms: client on 1 CPU (CLIENT=1), tier-0 suspected (.NET 8 multiplies the tier-up call-counting delay by TC_DelaySingleProcMultiplier=10 when the process is affinitized to one CPU: ~1 s, restarted at each new tier-0 JIT, longer than each BDN child's measurement); kept as a record, superseded by ../baseline/ (2-CPU client). See JOURNAL 73.
# csharp slice OPTIMISATION BENCHMARK (gen/opt_bench.sh), core grid (AK_CAMPAIGN_GRID=core, CAMPAIGN section 4.0)
# CONTAINER INSTRUMENTATION: not a campaign result (README 1.1, CAMPAIGN section 2); NOT GATED (gen/gate.sh not run); each codec process's Cases.Verify byte-identity pre-check is on, and every RPC call is checked (req 18)
# commit:     ec3429e5a1e675748031a59d3c762fe9b6db480f (clean)
# utc:        2026-10-04T08:23:00Z
# machine:    Intel(R) Xeon(R) Processor @ 2.80GHz; 4 logical CPUs; kernel 6.18.44-fc-v64; smt 0
# cpu sets:   CLIENT=1 SERVER=2,3 (taskset); AK_WORKERS=8 (core runtime workers, .NET thread pool min/max in the RPC client), server tokio workers AK_SERVER_THREADS=8
# .NET:       SDK 8.0.131; runtimes Microsoft.AspNetCore.App 8.0.31;Microsoft.NETCore.App 8.0.31; (Ubuntu noble-updates packages dotnet-sdk-8.0 8.0.131-0ubuntu1~24.04.1); target net8.0 Release; workstation concurrent GC, tiering/PGO at defaults
# rust:       rustc 1.94.1 (e408947bf 2026-03-25); cargo 1.94.1 (29ea6fb6a 2026-03-24)
# core:       target-core (rpc,init-guard) sha256 8932d20505b6d5f062a59e7a830a1df03f9705bfe8b3e5e63ef0296d5bd9b436
#             target-core-nounk (rpc,init-guard, --no-default-features) sha256 837687368933381fda3ca67cd0747072257c736a1cff029e4af5d4f483df5631
#             target-core-h2b (rpc,init-guard, h2-batch) sha256 a6faa4d6d8aacbf3f557b88e2ca6f01f2a3fdc8d19a08f22c92630288a12d53a
#             built by gen/build_core.sh from git archive HEAD ffi/poc/codec (last core commit fd69b0d6)
# BDN:        BenchmarkDotNet 0.15.8, DEFAULT toolchain (one child process per case, as the campaign), one launch (launch 1: full build first), merged runs (one per codec build; one per RPC run kind)
#             codec: --warmup 4 --rounds 6 --iteration-ms 40 (campaign: 10 / 5 / 100, 3 launches)
#             rpc:   --warmup 4 --rounds 6 --iteration-ms 100 (campaign: 10 / 5 / 100, 3 launches); >= 21 calls per caller thread before the first measured value (req 24: >= 20)
#             MemoryDiagnoser on (AK_BDN_MEMORY=1): one extra workload iteration per case after the actual stage, outside the job's clock
# grid:       codec: units incumbent-prod:default, core-ffi:retain, host-gen:retain (full build), core-ffi:no-unknown, host-gen:no-unknown (no-unknown build); encode-transport-hot and decode-read; 16 shapes (P7.1 decode only), Latin-1 and wide on P2.2, 7 U-* rows
#             rpc: transport armonik; stock h2: A, Bf (+ B a+read), Cf-retain (+ C-retain a+read), Ef-retain (+ E-retain a+read) on a+read, b (P2.2), c (P5.4), d (16 MiB) at k = 1 and 8; h2-batch: Cf-retain on c, d at k = 1, 8; pinned allocator: A, Cf-retain on c, d at k = 1
# dropped:    nothing of the core grid; calib not run; no plant controls
# allocator:  default (GLIBC_TUNABLES unset) for every process except the pinned-allocator subset (GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432)
# build: 23.2 s
# codec full: 199.1 s rc=0
# codec nounk: 142.1 s rc=0
# rpc server build: 0.2 s
# rpc server warm (2000): 127.8 s
# rpc stock: 153.4 s rc=0
# rpc h2-batch: 51.4 s rc=0
# rpc stock-pinned: 47.1 s rc=0
# total: 746.5 s
```

## Codec suite

Units and the labels the rows carry (arm, unknown_mode, build): `core-ffi no-unknown no-unknown`; `core-ffi retain full`; `host-gen no-unknown no-unknown`; `host-gen retain full`; `incumbent-prod default full`. `incumbent-prod` runs in the full build only (section 4.0). Payloads: SHAPES.md; `/latin1`, `/wide` = content sets on P2.2; U-* = corpus rows (content `corpus`). encode = `encode-transport-hot` (end state ii, the form the arm hands its transport, one hot graph); decode = `decode-read` (decode, then read every field).

### encode (encode-transport-hot): process CPU per op, microseconds

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 5.145 [5.096-5.201] | 1.415 [1.409-1.426] | 1.403 [1.391-1.428] | 3.674 [3.640-3.726] | 3.249 [3.233-3.258] |
| P1.2 | 1406 [1381-1429] | 328.8 [323.9-337.0] | 307.0 [303.9-308.0] | 797.2 [791.8-822.5] | 749.9 [746.3-818.1] |
| P1.3 | 43.56 [43.25-44.53] | 24.04 [23.96-24.07] | 22.19 [22.11-22.40] | 17.69 [17.29-18.39] | 17.56 [17.43-18.04] |
| P2.1 | 9.070 [9.043-9.111] | 2.544 [2.535-2.591] | 2.491 [2.466-2.510] | 4.637 [4.631-4.651] | 4.640 [4.606-4.664] |
| P2.2 | 5305 [5294-5387] | 1213 [1199-1237] | 1169 [1134-1212] | 2557 [2536-2636] | 2583 [2559-2647] |
| P2.2/latin1 | 6130 [6080-6178] | 1470 [1440-1485] | 1432 [1414-1474] | 2759 [2730-3012] | 2776 [2743-2819] |
| P2.2/wide | 6593 [6384-10314] * ^ | 1853 [1743-2174] | 1711 [1685-1895] | 3099 [3033-3292] | 3160 [3116-3207] |
| P2.3 | 3528 [3480-3569] | 892.0 [875.3-1064] | 862.1 [857.4-885.3] | 1596 [1542-1636] | 1548 [1537-1572] |
| P2.4 | 4357 [4346-4401] | 1923 [1890-1991] | 1408 [1384-1452] | 2020 [1971-2138] | 2069 [2032-2090] |
| P2.5 | 174.7 [171.9-194.3] | 39.88 [39.38-40.48] | 39.63 [39.13-42.81] | 79.64 [78.36-82.23] | 87.32 [79.76-89.16] |
| P3.1 | 106.8 [106.6-107.2] | 26.77 [26.20-26.85] | 25.18 [25.01-25.41] | 56.72 [56.60-58.54] | 58.24 [57.00-61.14] |
| P4.1 | 1041 [1034-1046] | 202.3 [197.9-219.3] | 197.2 [193.6-216.5] | 446.9 [445.7-452.8] | 451.3 [448.2-460.6] |
| P5.1 | 0.589 [0.584-0.598] | 0.297 [0.296-0.298] | 0.304 [0.295-0.324] | 0.727 [0.721-0.733] | 0.727 [0.720-0.742] |
| P5.2 | 2.132 [2.112-2.173] | 3.482 [3.477-3.504] | 3.470 [3.466-3.484] | 4.026 [4.006-4.158] | 3.975 [3.957-4.137] |
| P5.3 | 60.07 [59.67-61.20] | 143.8 [136.0-146.5] | 140.1 [136.6-141.2] | 134.4 [133.6-135.6] | 151.8 [151.2-152.3] |
| P5.4 | 266.4 [264.9-309.8] ^ | 556.6 [555.0-557.9] | 648.2 [635.3-664.7] | 629.6 [625.4-634.5] | 543.5 [541.7-558.9] |
| P6.1 | 3035 [3027-3064] | 149.7 [148.5-157.4] | 147.1 [145.5-150.1] | 915.0 [870.0-1051] ^ | 888.7 [860.6-926.6] |
| U-deep-u-repeated | 9.696 [9.657-9.723] | 2.341 [2.332-2.413] | 2.339 [2.316-2.429] | 4.410 [4.383-4.417] | 4.493 [4.453-4.525] |
| U-nested-before | 4.253 [4.226-4.350] | 0.609 [0.601-0.702] | 0.555 [0.554-0.561] | 1.237 [1.225-1.242] | 1.168 [1.150-1.177] |
| U-oneof-u-repeated | 1.693 [1.680-1.740] | 0.382 [0.375-0.411] | 0.353 [0.351-0.357] | 0.861 [0.835-0.877] | 0.856 [0.834-0.861] |
| U-wire-DualResponse-left-as-wt5 | 1.744 [1.735-1.784] | 0.558 [0.553-0.570] | 0.509 [0.504-0.516] | 1.160 [1.151-1.170] | 1.050 [1.036-1.059] |
| U-wire-ListMetricsResponse-batches-as-wt0 | 30.85 [30.43-31.62] | 4.211 [4.195-4.364] | 4.177 [4.152-4.207] | 9.139 [9.072-9.194] | 9.432 [9.328-9.724] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 10.62 [10.50-11.27] | 2.208 [2.197-2.218] | 2.115 [2.072-2.186] | 4.552 [4.539-4.623] | 4.497 [4.477-4.512] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 1.041 [1.036-1.171] | 0.341 [0.314-0.362] | 0.324 [0.314-0.350] | 0.763 [0.757-0.819] | 0.736 [0.718-0.752] |

### encode (encode-transport-hot): wall per op, microseconds

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 5.229 [5.134-5.541] | 1.433 [1.414-1.505] | 1.439 [1.402-1.843] * | 3.939 [3.716-4.263] | 3.251 [3.245-3.267] |
| P1.2 | 1428 [1384-1775] * | 336.6 [324.7-343.1] | 337.4 [307.0-346.0] | 810.4 [795.3-1004] * ^ | 765.7 [752.9-964.6] * ^ |
| P1.3 | 48.95 [43.37-50.24] | 24.07 [23.97-24.21] | 22.33 [22.16-23.13] | 18.74 [17.47-19.34] | 17.63 [17.46-19.83] |
| P2.1 | 10.03 [9.137-10.09] | 2.553 [2.538-2.680] | 2.541 [2.472-2.666] | 4.646 [4.641-4.657] | 4.646 [4.613-4.673] |
| P2.2 | 5335 [5298-5531] | 1244 [1204-1263] | 1174 [1135-1214] | 2561 [2537-2636] | 2589 [2571-2659] |
| P2.2/latin1 | 6256 [6085-6678] | 1484 [1446-1521] | 1443 [1416-1479] | 2770 [2734-3225] | 2790 [2747-2821] |
| P2.2/wide | 7170 [6448-10443] * ^ | 1981 [1764-2749] * | 1726 [1689-1997] | 3107 [3046-3990] * | 3186 [3124-3236] |
| P2.3 | 3562 [3483-3826] | 921.9 [880.7-1137] * | 882.3 [874.7-1079] | 1673 [1575-1888] | 1556 [1539-1879] |
| P2.4 | 4368 [4350-4414] | 1971 [1892-2197] | 1461 [1385-1849] * | 2293 [2000-2936] * ^ | 2208 [2076-2335] |
| P2.5 | 175.6 [172.0-204.9] | 40.04 [39.87-42.36] | 42.60 [39.81-75.20] * | 80.19 [78.42-114.2] * | 98.60 [79.78-100.2] |
| P3.1 | 107.0 [106.6-107.5] | 28.56 [26.21-29.46] | 25.44 [25.05-26.19] | 62.13 [56.95-96.14] * ^ | 59.72 [58.01-66.19] |
| P4.1 | 1137 [1044-1165] | 206.7 [198.2-267.7] * | 202.2 [196.8-223.8] | 447.4 [446.2-456.3] | 463.9 [454.8-580.4] * |
| P5.1 | 0.589 [0.584-0.686] | 0.297 [0.296-0.300] | 0.317 [0.296-0.363] | 0.743 [0.730-0.763] | 0.774 [0.726-0.873] |
| P5.2 | 2.161 [2.113-2.343] | 3.499 [3.483-3.859] | 3.480 [3.467-3.490] | 4.160 [4.027-5.665] * | 4.414 [3.970-5.789] * |
| P5.3 | 60.75 [60.03-64.14] | 144.3 [142.4-147.4] | 141.5 [139.4-164.7] ^ | 134.7 [133.7-136.0] | 152.1 [151.7-152.7] |
| P5.4 | 266.8 [265.3-310.5] ^ | 557.9 [556.0-560.1] | 681.1 [642.3-738.0] | 642.8 [628.3-650.5] | 545.1 [542.3-573.0] |
| P6.1 | 3048 [3039-3090] | 151.2 [148.6-190.9] * | 147.6 [145.8-150.8] | 924.7 [870.6-1118] * ^ | 891.0 [862.0-926.9] |
| U-deep-u-repeated | 10.71 [9.751-11.10] | 2.362 [2.347-2.942] * | 2.359 [2.317-2.436] | 4.419 [4.385-4.437] | 4.852 [4.567-5.106] |
| U-nested-before | 4.273 [4.239-4.421] | 0.611 [0.603-0.768] * | 0.612 [0.568-0.629] | 1.270 [1.238-1.362] | 1.235 [1.159-1.282] |
| U-oneof-u-repeated | 1.698 [1.686-1.997] ^ | 0.391 [0.376-0.437] | 0.386 [0.354-0.425] ^ | 0.872 [0.835-0.895] | 0.863 [0.838-1.013] |
| U-wire-DualResponse-left-as-wt5 | 1.904 [1.776-2.611] * | 0.569 [0.554-0.683] | 0.510 [0.504-0.516] | 1.167 [1.155-1.180] | 1.151 [1.052-1.174] |
| U-wire-ListMetricsResponse-batches-as-wt0 | 31.60 [30.64-33.30] | 4.454 [4.201-5.120] | 4.199 [4.164-4.264] | 9.300 [9.081-13.44] * | 10.24 [9.383-21.59] * |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 10.85 [10.62-15.26] * | 2.226 [2.211-2.271] | 2.134 [2.079-2.281] | 4.595 [4.577-5.010] | 4.515 [4.480-4.670] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 1.045 [1.036-1.284] | 0.353 [0.315-0.370] | 0.371 [0.314-0.436] * | 0.766 [0.761-0.883] | 0.738 [0.720-0.754] |

### encode (encode-transport-hot): allocated bytes per op, and Gen0/Gen1/Gen2 per 1,000 ops (MemoryDiagnoser iteration: ops)

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 0 B; 0/0/0 (7767 ops) | 0 B; 0/0/0 (27606 ops) | 0 B; 0/0/0 (28798 ops) | 0 B; 0/0/0 (10790 ops) | 0 B; 0/0/0 (12215 ops) |
| P1.2 | 0 B; 0/0/0 (29 ops) | 0 B; 0/0/0 (116 ops) | 0 B; 0/0/0 (115 ops) | 0 B; 0/0/0 (49 ops) | 0 B; 0/0/0 (53 ops) |
| P1.3 | 0 B; 0/0/0 (828 ops) | 0 B; 0/0/0 (1661 ops) | 0 B; 0/0/0 (1804 ops) | 0 B; 0/0/0 (2325 ops) | 0 B; 0/0/0 (2280 ops) |
| P2.1 | 56 B; 0/0/0 (4416 ops) | 0 B; 0/0/0 (15556 ops) | 0 B; 0/0/0 (16101 ops) | 128 B; 0/0/0 (8532 ops) | 128 B; 0/0/0 (8516 ops) |
| P2.2 | 28,000 B; 0/0/0 (7 ops) | 0 B; 0/0/0 (33 ops) | 0 B; 0/0/0 (34 ops) | 64,000 B; 0/0/0 (14 ops) | 64,000 B; 0/0/0 (14 ops) |
| P2.2/latin1 | 28,000 B; 0/0/0 (6 ops) | 0 B; 0/0/0 (26 ops) | 24 B; 0/0/0 (28 ops) | 64,000 B; 0/0/0 (13 ops) | 64,000 B; 0/0/0 (13 ops) |
| P2.2/wide | 28,000 B; 0/0/0 (4 ops) | 0 B; 0/0/0 (22 ops) | 0 B; 0/0/0 (23 ops) | 64,000 B; 0/0/0 (12 ops) | 64,000 B; 0/0/0 (12 ops) |
| P2.3 | 7,000 B; 0/0/0 (10 ops) | 0 B; 0/0/0 (46 ops) | 0 B; 0/0/0 (45 ops) | 16,000 B; 0/0/0 (25 ops) | 16,000 B; 0/0/0 (24 ops) |
| P2.4 | 4,480 B; 0/0/0 (9 ops) | 32 B; 0/0/0 (21 ops) | 0 B; 0/0/0 (27 ops) | 10,240 B; 0/0/0 (18 ops) | 10,240 B; 0/0/0 (18 ops) |
| P2.5 | 1,120 B; 0/0/0 (223 ops) | 0 B; 0/0/0 (929 ops) | 0 B; 0/0/0 (913 ops) | 2,560 B; 0/0/0 (471 ops) | 2,560 B; 0/0/0 (398 ops) |
| P3.1 | 0 B; 0/0/0 (371 ops) | 0 B; 0/0/0 (1419 ops) | 0 B; 0/0/0 (1575 ops) | 0 B; 0/0/0 (671 ops) | 0 B; 0/0/0 (697 ops) |
| P4.1 | 11,200 B; 0/0/0 (35 ops) | 0 B; 0/0/0 (197 ops) | 0 B; 0/0/0 (205 ops) | 25,600 B; 0/0/0 (84 ops) | 25,600 B; 0/0/0 (83 ops) |
| P5.1 | 0 B; 0/0/0 (67593 ops) | 0 B; 0/0/0 (134207 ops) | 0 B; 0/0/0 (134532 ops) | 0 B; 0/0/0 (54774 ops) | 0 B; 0/0/0 (56127 ops) |
| P5.2 | 0 B; 0/0/0 (18393 ops) | 0 B; 0/0/0 (11127 ops) | 0 B; 0/0/0 (11391 ops) | 0 B; 0/0/0 (9807 ops) | 0 B; 0/0/0 (9986 ops) |
| P5.3 | 0 B; 0/0/0 (520 ops) | 0 B; 0/0/0 (190 ops) | 0 B; 0/0/0 (309 ops) | 0 B; 0/0/0 (299 ops) | 0 B; 0/0/0 (260 ops) |
| P5.4 | 0 B; 0/0/0 (151 ops) | 0 B; 0/0/0 (71 ops) | 0 B; 0/0/0 (69 ops) | 0 B; 0/0/0 (63 ops) | 9 B; 0/0/0 (73 ops) |
| P6.1 | 0 B; 0/0/0 (12 ops) | 0 B; 0/0/0 (270 ops) | 0 B; 0/0/0 (273 ops) | 0 B; 0/0/0 (44 ops) | 0 B; 0/0/0 (45 ops) |
| U-deep-u-repeated | 280 B; 0/0/0 (3650 ops) | 0 B; 0/0/0 (17073 ops) | 0 B; 0/0/0 (16708 ops) | 128 B; 0/0/0 (8948 ops) | 128 B; 0/0/0 (8380 ops) |
| U-nested-before | 168 B; 0/0/0 (9296 ops) | 0 B; 0/0/0 (65656 ops) | 0 B; 0/0/0 (72284 ops) | 0 B; 0/0/0 (32375 ops) | 0 B; 0/0/0 (34573 ops) |
| U-oneof-u-repeated | 168 B; 0/0/0 (23745 ops) | 0 B; 0/0/0 (106003 ops) | 0 B; 0/0/0 (102167 ops) | 0 B; 0/0/0 (47173 ops) | 0 B; 0/0/0 (47815 ops) |
| U-wire-DualResponse-left-as-wt5 | 112 B; 0/0/0 (22814 ops) | 0 B; 0/0/0 (71257 ops) | 0 B; 0/0/0 (79115 ops) | 0 B; 0/0/0 (32917 ops) | 0 B; 0/0/0 (37925 ops) |
| U-wire-ListMetricsResponse-batches-as-wt0 | 113 B; 0/0/0 (1297 ops) | 0 B; 0/0/0 (9187 ops) | 0 B; 0/0/0 (9501 ops) | 0 B; 0/0/0 (4392 ops) | 0 B; 0/0/0 (3940 ops) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 224 B; 0/0/0 (3580 ops) | 0 B; 0/0/0 (18139 ops) | 0 B; 0/0/0 (18930 ops) | 256 B; 0/0/0 (8770 ops) | 256 B; 0/0/0 (8820 ops) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 112 B; 0/0/0 (37927 ops) | 0 B; 0/0/0 (127729 ops) | 0 B; 0/0/0 (124303 ops) | 0 B; 0/0/0 (52670 ops) | 0 B; 0/0/0 (54939 ops) |

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
| P1.1 | 6.319 [6.255-6.757] | 2.803 [2.792-2.919] | 2.768 [2.695-2.870] | 4.022 [4.008-4.105] | 4.133 [4.033-4.278] |
| P1.2 | 1509 [1499-1524] | 540.2 [527.4-570.3] | 513.0 [503.0-528.4] | 1067 [1042-1118] | 1045 [1039-1070] |
| P1.3 | 67.61 [61.29-71.82] | 34.56 [33.75-35.25] | 34.26 [34.06-34.60] | 30.16 [29.99-30.44] | 31.15 [30.85-31.49] |
| P2.1 | 9.916 [9.772-10.11] | 5.723 [5.563-5.961] | 5.531 [5.500-5.619] | 6.421 [6.344-6.758] | 6.283 [6.232-6.336] |
| P2.2 | 5575 [5265-5898] | 3044 [2971-3257] | 2937 [2883-3065] | 4284 [4075-4515] | 4147 [3922-4250] |
| P2.2/latin1 | 5951 [5942-5999] | 3896 [3678-3910] | 3655 [3510-3758] | 4308 [4208-4352] | 4669 [4445-4743] |
| P2.2/wide | 7088 [6947-7364] | 4406 [4281-4454] | 4619 [4418-4706] | 5136 [5063-5244] | 5007 [4964-5051] |
| P2.3 | 3612 [3521-3648] | 1984 [1928-2134] | 1924 [1888-2026] | 2684 [2631-2739] | 2521 [2496-2654] |
| P2.4 | 4439 [4415-4541] | 2546 [2456-2650] | 2560 [2518-2622] | 3412 [3325-3472] | 3317 [3272-3363] |
| P2.5 | 181.1 [179.4-186.6] | 92.78 [91.29-101.4] | 94.37 [93.40-96.56] | 119.8 [119.3-121.0] | 124.4 [121.9-125.3] |
| P3.1 | 117.3 [116.1-153.9] * | 48.69 [48.27-49.49] | 48.33 [47.90-48.85] | 84.83 [82.82-86.94] | 79.66 [79.48-80.62] |
| P4.1 | 906.5 [889.7-940.0] | 484.5 [477.6-509.6] | 489.7 [483.3-528.2] | 606.3 [603.3-664.0] | 624.6 [599.1-667.7] |
| P5.1 | 0.611 [0.596-0.625] | 0.677 [0.653-0.693] | 0.792 [0.642-1.031] * | 0.324 [0.320-0.331] | 0.308 [0.306-0.310] |
| P5.2 | 5.999 [5.837-6.202] | 5.564 [5.513-5.619] | 5.746 [5.605-6.566] | 5.134 [5.054-5.287] | 5.436 [5.181-5.772] |
| P5.3 | 261.0 [224.8-315.7] * ^ | 289.6 [266.7-296.2] | 251.4 [236.5-323.6] * | 264.0 [251.6-291.5] | 282.8 [257.2-334.2] * ^ |
| P5.4 | 946.7 [940.3-963.3] | 945.0 [934.6-992.1] | 1366 [1336-1375] | 875.7 [871.4-894.7] | 1242 [1225-1261] |
| P6.1 | 2224 [2208-2294] | 687.3 [680.1-696.4] | 686.4 [675.2-702.9] | 1454 [1433-1546] | 1453 [1446-1529] |
| P7.1 | 2.234 [2.219-2.253] | 1.289 [1.273-1.318] | 1.278 [1.256-1.297] | 1.305 [1.285-1.362] | 1.260 [1.244-1.274] |
| U-deep-u-repeated | 9.958 [9.876-10.70] | 5.718 [5.668-5.846] | 5.108 [5.056-5.134] | 6.216 [6.182-6.283] | 5.981 [5.966-6.334] |
| U-nested-before | 3.618 [3.601-3.694] | 1.850 [1.806-1.890] | 1.207 [1.202-1.214] | 1.703 [1.682-1.749] | 1.425 [1.420-1.434] |
| U-oneof-u-repeated | 1.440 [1.415-1.514] | 1.179 [1.174-1.183] | 0.803 [0.793-0.814] | 0.701 [0.682-0.724] | 0.640 [0.631-0.651] |
| U-wire-DualResponse-left-as-wt5 | 1.978 [1.950-2.082] | 1.469 [1.465-1.477] | 1.072 [1.050-1.142] | 0.974 [0.964-0.992] | 0.936 [0.932-0.951] |
| U-wire-ListMetricsResponse-batches-as-wt0 | 22.65 [22.59-22.87] | 8.092 [7.940-8.534] | 7.639 [7.592-8.121] | 14.64 [14.49-17.97] | 14.66 [14.46-14.82] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 9.350 [9.263-9.452] | 5.149 [5.101-5.215] | 5.092 [4.777-6.733] * | 5.938 [5.891-6.033] | 5.936 [5.895-5.971] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 0.876 [0.857-0.892] | 0.990 [0.961-1.077] | 0.625 [0.617-0.631] | 0.374 [0.367-0.379] | 0.338 [0.331-0.365] |

### decode-read: wall per op, microseconds

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 6.385 [6.268-6.874] | 2.835 [2.808-2.932] | 2.842 [2.721-3.284] | 4.129 [4.058-4.286] | 4.657 [4.076-5.932] * |
| P1.2 | 1537 [1504-1596] | 615.2 [531.4-729.8] * | 569.2 [518.9-610.7] | 1083 [1050-1520] * | 1059 [1040-1075] |
| P1.3 | 69.28 [61.85-74.48] | 34.98 [33.87-36.96] | 34.42 [34.17-35.10] | 30.29 [30.04-30.51] | 31.45 [30.97-32.02] |
| P2.1 | 9.930 [9.789-10.19] | 5.997 [5.586-6.799] | 5.538 [5.517-5.632] | 6.444 [6.356-6.764] | 6.298 [6.237-6.340] |
| P2.2 | 5897 [5273-7021] * | 3054 [2984-4247] * | 2954 [2884-3092] | 4311 [4080-8838] * ^ | 4439 [3940-6182] * |
| P2.2/latin1 | 6000 [5952-6500] | 4080 [3752-4625] | 3666 [3515-3770] | 4613 [4218-4911] | 4725 [4486-4799] |
| P2.2/wide | 7100 [6962-7386] | 4411 [4351-4496] | 4667 [4424-4712] | 6024 [5785-6542] | 5017 [4968-5134] |
| P2.3 | 3628 [3540-4905] * | 2181 [2020-3409] * | 2117 [1924-2216] | 2831 [2679-3121] | 2557 [2508-3007] ^ |
| P2.4 | 4449 [4418-4544] | 2899 [2684-3250] ^ | 2571 [2524-2740] | 3469 [3332-4023] | 3587 [3483-3877] |
| P2.5 | 184.6 [181.3-226.0] | 93.35 [91.36-157.2] * ^ | 94.61 [93.63-96.62] | 120.1 [119.6-121.1] | 127.2 [122.2-131.1] |
| P3.1 | 117.5 [116.2-155.0] * | 50.06 [49.12-64.82] * | 53.27 [48.47-54.13] | 90.81 [82.96-109.4] * | 79.93 [79.70-80.92] |
| P4.1 | 909.3 [893.4-1096] ^ | 546.3 [484.6-762.3] * | 533.3 [497.5-604.2] ^ | 608.5 [606.8-665.7] | 651.0 [602.7-693.5] |
| P5.1 | 0.627 [0.622-0.750] | 0.690 [0.654-0.695] | 0.910 [0.647-1.034] * | 0.327 [0.321-0.342] | 0.335 [0.310-0.352] |
| P5.2 | 6.236 [5.955-7.035] | 5.568 [5.517-5.630] | 5.880 [5.631-6.976] | 5.275 [5.137-5.340] | 5.443 [5.190-5.783] |
| P5.3 | 261.5 [225.0-316.0] * ^ | 295.7 [273.6-303.4] | 253.5 [236.7-324.6] * | 272.6 [256.3-309.0] | 283.6 [257.5-338.1] * ^ |
| P5.4 | 956.8 [941.5-992.4] | 960.6 [935.9-1371] * | 1370 [1343-1380] | 909.0 [872.2-1045] | 1263 [1245-1307] |
| P6.1 | 2305 [2246-3212] * | 693.2 [680.5-704.4] | 710.4 [675.6-734.3] | 1491 [1454-1598] | 1465 [1451-4346] * |
| P7.1 | 2.238 [2.223-2.292] | 1.294 [1.281-1.386] | 1.280 [1.261-1.306] | 1.452 [1.374-1.806] * | 1.419 [1.245-1.442] |
| U-deep-u-repeated | 9.985 [9.896-10.71] | 5.722 [5.667-6.154] | 5.138 [5.077-5.167] | 6.231 [6.189-6.287] | 6.017 [5.982-6.350] |
| U-nested-before | 3.627 [3.609-3.780] | 1.913 [1.816-2.226] ^ | 1.208 [1.205-1.218] | 1.757 [1.683-1.907] | 1.432 [1.421-1.441] |
| U-oneof-u-repeated | 1.477 [1.419-1.801] * | 1.189 [1.180-1.216] | 0.805 [0.795-0.814] | 0.758 [0.721-0.899] | 0.726 [0.682-0.733] |
| U-wire-DualResponse-left-as-wt5 | 2.029 [1.969-2.380] | 1.472 [1.467-1.482] | 1.226 [1.134-1.301] | 1.069 [0.978-1.144] | 0.942 [0.935-0.963] |
| U-wire-ListMetricsResponse-batches-as-wt0 | 24.40 [23.15-26.32] | 8.303 [7.958-8.634] | 7.685 [7.626-8.128] | 15.14 [14.62-18.54] * | 14.70 [14.47-16.23] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 9.376 [9.299-9.474] | 5.161 [5.118-5.429] | 5.572 [4.837-7.936] * | 5.957 [5.897-6.055] | 5.948 [5.905-6.001] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 0.884 [0.865-1.278] * ^ | 1.120 [0.970-1.348] * ^ | 0.636 [0.621-0.697] | 0.376 [0.368-0.393] | 0.342 [0.332-0.405] |

### decode-read: allocated bytes per op, and Gen0/Gen1/Gen2 per 1,000 ops (MemoryDiagnoser iteration: ops)

| payload | incumbent-prod | core-ffi retain | core-ffi no-unknown | host-gen retain | host-gen no-unknown |
|---|---:|---:|---:|---:|---:|
| P1.1 | 2,888 B; 0.16/0/0 (6306 ops) | 2,728 B; 0.14/0/0 (14354 ops) | 2,728 B; 0.13/0/0 (14849 ops) | 2,728 B; 0.1/0/0 (9597 ops) | 2,728 B; 0.1/0/0 (9935 ops) |
| P1.2 | 697,943 B; 0/0/0 (23 ops) | 665,984 B; 32.26/16.13/0 (62 ops) | 665,984 B; 27.78/13.89/0 (72 ops) | 665,984 B; 32.26/0/0 (31 ops) | 665,984 B; 28.57/0/0 (35 ops) |
| P1.3 | 39,568 B; 1.56/0/0 (640 ops) | 39,624 B; 1.7/0/0 (1174 ops) | 39,624 B; 1.72/0/0 (1161 ops) | 39,624 B; 2.23/0/0 (1344 ops) | 39,624 B; 1.56/0/0 (1280 ops) |
| P2.1 | 4,904 B; 0.25/0/0 (3930 ops) | 4,544 B; 0.15/0/0 (6819 ops) | 4,544 B; 0.14/0/0 (7238 ops) | 4,544 B; 0.16/0/0 (6164 ops) | 4,544 B; 0.16/0/0 (6225 ops) |
| P2.2 | 2,344,528 B; 0/0/0 (6 ops) | 2,180,584 B; 83.33/0/0 (12 ops) | 2,180,584 B; 83.33/0/0 (12 ops) | 2,180,584 B; 125/0/0 (8 ops) | 2,180,584 B; 111.11/0/0 (9 ops) |
| P2.2/latin1 | 2,344,528 B; 0/0/0 (6 ops) | 2,180,584 B; 111.11/0/0 (9 ops) | 2,180,618 B; 100/0/0 (10 ops) | 2,180,632 B; 0/0/0 (7 ops) | 2,180,584 B; 125/0/0 (8 ops) |
| P2.2/wide | 2,344,528 B; 0/0/0 (4 ops) | 2,180,584 B; 111.11/0/0 (9 ops) | 2,180,584 B; 125/0/0 (8 ops) | 2,180,584 B; 0/0/0 (7 ops) | 2,180,584 B; 0/0/0 (7 ops) |
| P2.3 | 2,098,104 B; 100/0/0 (10 ops) | 2,101,160 B; 111.11/55.56/0 (18 ops) | 2,101,160 B; 62.5/0/0 (16 ops) | 2,101,160 B; 83.33/0/0 (12 ops) | 2,101,160 B; 66.67/0/0 (15 ops) |
| P2.4 | 3,287,832 B; 142.86/0/0 (7 ops) | 3,275,728 B; 153.85/76.92/0 (13 ops) | 3,275,728 B; 176.47/117.65/0 (17 ops) | 3,275,728 B; 125/0/0 (8 ops) | 3,275,728 B; 111.11/0/0 (9 ops) |
| P2.5 | 87,976 B; 4.65/0/0 (215 ops) | 81,472 B; 2.42/0/0 (414 ops) | 81,472 B; 4.71/0/0 (425 ops) | 81,472 B; 3.09/0/0 (324 ops) | 81,472 B; 3.29/0/0 (304 ops) |
| P3.1 | 49,664 B; 0/0/0 (312 ops) | 53,880 B; 2.43/0/0 (823 ops) | 53,880 B; 2.42/0/0 (826 ops) | 53,880 B; 2.36/0/0 (424 ops) | 53,880 B; 2.72/0/0 (368 ops) |
| P4.1 | 403,704 B; 0/0/0 (32 ops) | 363,760 B; 14.93/0/0 (67 ops) | 363,760 B; 16.39/0/0 (61 ops) | 363,760 B; 20.41/0/0 (49 ops) | 363,760 B; 0/0/0 (47 ops) |
| P5.1 | 368 B; 0.02/0/0 (60977 ops) | 336 B; 0.02/0/0 (59099 ops) | 336 B; 0.02/0/0 (63102 ops) | 336 B; 0.02/0/0 (114080 ops) | 336 B; 0.02/0/0 (130848 ops) |
| P5.2 | 65,864 B; 3.79/0.66/0 (6065 ops) | 65,832 B; 3.73/0.69/0 (7237 ops) | 65,832 B; 3.73/0.68/0 (5893 ops) | 65,832 B; 3.78/0.65/0 (7664 ops) | 65,832 B; 3.7/0.64/0 (7840 ops) |
| P5.3 | 1,049,109 B; 48.48/48.48/48.48 (165 ops) | 1,049,028 B; 44.44/44.44/44.44 (135 ops) | 1,049,015 B; 44.87/44.87/44.87 (156 ops) | 1,048,992 B; 45.45/45.45/45.45 (176 ops) | 1,049,169 B; 62.5/62.5/62.5 (144 ops) |
| P5.4 | 4,194,649 B; 50/50/50 (20 ops) | 4,194,618 B; 52.63/52.63/52.63 (19 ops) | 4,194,615 B; 95.24/95.24/95.24 (21 ops) | 4,194,600 B; 0/0/0 (17 ops) | 4,194,633 B; 62.5/62.5/62.5 (32 ops) |
| P6.1 | 356,240 B; 0/0/0 (16 ops) | 466,696 B; 17.24/0/0 (58 ops) | 466,696 B; 17.86/0/0 (56 ops) | 466,696 B; 0/0/0 (26 ops) | 466,696 B; 0/0/0 (26 ops) |
| P7.1 | 776 B; 0/0/0 (17732 ops) | 712 B; 0.03/0/0 (30892 ops) | 712 B; 0.03/0/0 (30439 ops) | 712 B; 0.03/0/0 (28784 ops) | 712 B; 0.04/0/0 (26688 ops) |
| U-deep-u-repeated | 5,000 B; 0.26/0/0 (3894 ops) | 4,152 B; 0.15/0/0 (6703 ops) | 4,096 B; 0.13/0/0 (7784 ops) | 4,192 B; 0.16/0/0 (6373 ops) | 4,096 B; 0.15/0/0 (6541 ops) |
| U-nested-before | 2,688 B; 0.09/0/0 (10895 ops) | 904 B; 0.05/0/0 (21939 ops) | 784 B; 0.03/0/0 (32726 ops) | 1,384 B; 0.04/0/0 (22805 ops) | 784 B; 0.04/0/0 (28112 ops) |
| U-oneof-u-repeated | 904 B; 0.04/0/0 (27814 ops) | 400 B; 0/0/0 (33208 ops) | 352 B; 0.02/0/0 (50361 ops) | 440 B; 0.02/0/0 (58419 ops) | 352 B; 0.02/0/0 (61472 ops) |
| U-wire-DualResponse-left-as-wt5 | 984 B; 0.06/0/0 (17955 ops) | 568 B; 0/0/0 (27250 ops) | 536 B; 0.03/0/0 (32970 ops) | 568 B; 0.02/0/0 (40528 ops) | 536 B; 0.02/0/0 (42624 ops) |
| U-wire-ListMetricsResponse-batches-as-wt0 | 4,072 B; 0/0/0 (1665 ops) | 4,776 B; 0.2/0/0 (4884 ops) | 4,744 B; 0.19/0/0 (5199 ops) | 4,776 B; 0/0/0 (2750 ops) | 4,744 B; 0/0/0 (2364 ops) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 4,504 B; 0.24/0/0 (4190 ops) | 3,720 B; 0.13/0/0 (7712 ops) | 3,688 B; 0.13/0/0 (7507 ops) | 3,720 B; 0.15/0/0 (6743 ops) | 3,688 B; 0.16/0/0 (6093 ops) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 728 B; 0.02/0/0 (42644 ops) | 344 B; 0/0/0 (39891 ops) | 312 B; 0.02/0/0 (60398 ops) | 344 B; 0.02/0/0 (106768 ops) | 312 B; 0.01/0/0 (107184 ops) |

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
| A | stock / default | 1 | 7674 [7327-8332] | 7422 [7355-7879] | 7751 [7480-8399] | 2,355,208; 90.9/0/0 (11) | 0.1 | 0/0/0/0 | 1 |
| A | stock / default | 8 | 7918 [7549-10153] * | 7791 [7502-10171] * | 7980 [7571-10173] * | 2,371,835; 125/93.8/0 (32) | 26.8 | 0/0/0/0 | 1 |
| B | stock / default | 1 | 6110 [6011-6543] | 6031 [5922-6263] | 6296 [6251-8506] * | 2,344,552; 76.9/0/0 (13) | 0.4 | 2/0/0/0 | 0 |
| B | stock / default | 8 | 8143 [6886-8851] | 7793 [6883-8777] | 8164 [6908-8979] * | 2,344,531; 125/93.8/0 (32) | 36.4 | 2/0/0/0 | 0 |
| C-retain | stock / default | 1 | 3705 [3634-5004] * | 3681 [3532-3907] | 3853 [3829-5214] * | 2,180,623; 90.9/45.5/0 (22) | 0.0 | 6/3501/0/1 | 0 |
| C-retain | stock / default | 8 | 4305 [4078-4673] | 4306 [4091-4676] | 4319 [4125-4702] | 2,180,598; 125/93.8/0 (32) | 24.7 | 6/3501/0/1 | 4 |
| E-retain | stock / default | 1 | 5159 [4924-5507] | 4847 [4770-4900] | 5451 [5207-5706] | 2,180,608; 105.3/52.6/0 (19) | 0.1 | 2/0/0/0 | 1 |
| E-retain | stock / default | 8 | 5366 [5134-6272] | 5325 [5092-6044] | 5375 [5156-6317] | 2,180,587; 125/93.8/0 (32) | 6.2 | 2/0/0/0 | 3 |

### direction b (P2.2)

| cell | h2 / alloc | k | task-clock CPU / call | process CPU / call | wall / call | alloc B / call; Gen0/1/2 per 1k calls | minflt / call | crossings fwd/rev/grow/reset | client softirq ticks |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A | stock / default | 1 | 7328 [6901-7595] | 7201 [6915-7321] | 9078 [8520-9340] | 581,720; 0/0/0 (11) | 0.0 | 0/0/0/0 | 0 |
| A | stock / default | 8 | 6815 [6634-7058] | 6718 [6533-6931] | 7282 [6779-7504] | 581,447; 0/0/0 (32) | 48.6 | 0/0/0/0 | 3 |
| Bf | stock / default | 1 | 6385 [5881-8305] * | 6123 [5865-6692] | 8596 [7938-10873] * | 28,056; 0/0/0 (11) | 0.0 | 2/0/0/0 | 0 |
| Bf | stock / default | 8 | 6361 [5949-7155] | 6108 [5932-6815] | 6787 [6358-8066] * | 28,035; 0/0/0 (32) | 0.0 | 2/0/0/0 | 1 |
| Cf-retain | stock / default | 1 | 2291 [1921-2420] | 2049 [1897-2248] | 4293 [4000-5197] * | 85; 0/0/0 (25) | 0.0 | 2505/2501/0/0 | 1 |
| Cf-retain | stock / default | 8 | 2025 [1896-2303] | 1976 [1888-2295] | 2651 [2242-3034] * | 51; 0/0/0 (32) | 0.0 | 2505/2501/0/0 | 3 |
| Ef-retain | stock / default | 1 | 2991 [2942-3059] | 2976 [2945-3035] | 4831 [4790-4863] | 64,090; 0/0/0 (19) | 0.0 | 2/0/0/0 | 1 |
| Ef-retain | stock / default | 8 | 3348 [3133-3549] | 3118 [3084-3211] | 3917 [3825-4510] | 64,051; 0/0/0 (32) | 0.0 | 2/0/0/0 | 1 |

### direction c (P5.4)

| cell | h2 / alloc | k | task-clock CPU / call | process CPU / call | wall / call | alloc B / call; Gen0/1/2 per 1k calls | minflt / call | crossings fwd/rev/grow/reset | client softirq ticks |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A | stock / default | 1 | 8636 [6919-9795] * | 8687 [6818-9146] * | 9652 [7466-11106] * ^ | 4,242,010; 0/0/0 (9) | 454.7 | 0/0/0/0 | 8 |
| A | stock / default | 8 | 5718 [5284-6450] | 5757 [5225-6343] | 6701 [6291-7287] | 4,242,113; 0/0/0 (32) | 221.2 | 0/0/0/0 | 8 |
| A | stock / **pinned** | 1 | 9224 [7034-11617] * | 9361 [7093-11446] * | 9597 [8002-12051] * | 4,241,962; 0/0/0 (10) | 99.2 | 0/0/0/0 | 9 |
| Bf | stock / default | 1 | 3422 [3247-3758] | 3389 [3191-3604] | 5675 [5058-8316] * | 160; 0/0/0 (14) | 0.1 | 2/0/0/0 | 5 |
| Bf | stock / default | 8 | 4007 [3618-4097] | 3772 [3511-4016] | 5901 [5497-6672] | 126; 0/0/0 (32) | 0.1 | 2/0/0/0 | 13 |
| Cf-retain | **h2-batch** / default | 1 | 3600 [3162-3921] | 3463 [3048-3731] | 5927 [5170-6297] | 72; 0/0/0 (17) | 0.0 | 4/0/0/0 | 4 |
| Cf-retain | **h2-batch** / default | 8 | 3278 [3004-4244] * | 3236 [2957-3769] * | 5372 [5203-6353] | 51; 0/0/0 (32) | 0.1 | 4/0/0/0 | 11 |
| Cf-retain | stock / default | 1 | 3122 [2799-3528] | 2993 [2767-3345] | 5429 [4203-5873] * | 72; 0/0/0 (14) | 0.1 | 4/0/0/0 | 3 |
| Cf-retain | stock / default | 8 | 2921 [2795-2940] | 2838 [2766-2878] | 5039 [4746-6214] * ^ | 51; 0/0/0 (32) | 0.0 | 4/0/0/0 | 11 |
| Cf-retain | stock / **pinned** | 1 | 3525 [3116-4457] * | 3398 [3100-4242] * | 6324 [4994-8514] * | 72; 0/0/0 (17) | 0.0 | 4/0/0/0 | 7 |
| Ef-retain | stock / default | 1 | 3105 [2756-3487] ^ | 2949 [2720-3190] | 6041 [4269-6577] * | 136; 0/0/0 (23) | 0.0 | 2/0/0/0 | 6 |
| Ef-retain | stock / default | 8 | 3635 [3363-4295] * | 3528 [3342-4270] * | 5540 [5233-6016] | 115; 0/0/0 (32) | 0.1 | 2/0/0/0 | 15 |

### direction d (stream-16MiB)

| cell | h2 / alloc | k | task-clock CPU / call | process CPU / call | wall / call | alloc B / call; Gen0/1/2 per 1k calls | minflt / call | crossings fwd/rev/grow/reset | client softirq ticks |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A | stock / default | 1 | 29631 [27218-31300] | 29657 [27258-31341] | 29683 [27267-31374] | 16,957,280; 0/0/0 (4) | 1904.1 | 0/0/0/0 | 14 |
| A | stock / default | 8 | 20479 [19054-28272] * | 20318 [18594-26761] * | 21331 [20396-29765] * | 16,960,161; 62.5/62.5/62.5 (32) | 38.8 | 0/0/0/0 | 63 |
| A | stock / **pinned** | 1 | 34364 [31143-35926] | 33741 [31190-35546] | 37136 [35654-39387] | 16,957,350; 0/0/0 (4) | 3048.2 | 0/0/0/0 | 17 |
| Bf | stock / default | 1 | 13147 [11837-22042] * | 12426 [11730-16970] * | 14165 [12656-23411] * | 848; 0/0/0 (7) | 0.1 | 12/0/0/0 | 8 |
| Bf | stock / default | 8 | 17053 [15534-18189] | 16536 [15158-17588] | 19686 [17742-20471] | 827; 0/0/0 (32) | 0.3 | 12/0/0/0 | 48 |
| Cf-retain | **h2-batch** / default | 1 | 13434 [12305-14376] | 12815 [12315-14226] | 15027 [14225-16020] | 336; 0/0/0 (7) | 0.1 | 28/0/0/0 | 12 |
| Cf-retain | **h2-batch** / default | 8 | 15690 [14446-17027] | 15145 [13924-15944] | 19064 [17161-22233] * | 315; 0/0/0 (32) | 0.3 | 28/0/0/0 | 54 |
| Cf-retain | stock / default | 1 | 11282 [10482-11574] | 10708 [10359-11100] | 12980 [11906-14277] | 336; 0/0/0 (7) | 0.1 | 28/0/0/0 | 13 |
| Cf-retain | stock / default | 8 | 14076 [13793-15069] | 13628 [12755-14818] | 16954 [15643-18498] | 315; 0/0/0 (32) | 0.0 | 28/0/0/0 | 59 |
| Cf-retain | stock / **pinned** | 1 | 11936 [11609-13082] | 11916 [11577-12888] | 13833 [12652-15215] | 336; 0/0/0 (7) | 0.0 | 28/0/0/0 | 10 |
| Ef-retain | stock / default | 1 | 11320 [10634-13719] * | 11304 [10651-11756] | 13043 [11765-14770] | 890; 0/0/0 (8) | 0.1 | 12/0/0/0 | 9 |
| Ef-retain | stock / default | 8 | 16080 [14034-17469] | 15955 [14061-16884] | 17739 [15870-19102] | 827; 0/0/0 (32) | 0.2 | 12/0/0/0 | 53 |

