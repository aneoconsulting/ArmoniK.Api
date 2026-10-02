# backward-encode: encode tables (CONTAINER INSTRUMENTATION, not campaign results)

```
# backward-encode measurement, 2026-10-02T15:07:47Z. CONTAINER INSTRUMENTATION, not a campaign result.
# commit 1c9d8981665e98b9a71225ce1530fa04f756e94a + uncommitted changes in poc/rust or poc/codec
# worktree /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-bwd: 1c9d8981665e98b9a71225ce1530fa04f756e94a + backward-encode.patch ( 18 files changed, 8361 insertions(+), 5319 deletions(-))
# rustc 1.94.1 (e408947bf 2026-03-25); nproc 4
# cpu        Intel(R) Xeon(R) Processor @ 2.10GHz; kernel 6.18.44-fc-v51; smt notsupported (active 0); no_turbo n/a; governor cpu1 n/a; scaling min/max cpu1 n/a/n/a kHz
# isolation  cmdline: isolated='' nohz_full='n/a'
# cgroups    cpuset.cpus.effective:; this driver's cgroup /: n/a; root cpuset.cpus.isolated='n/a'; this driver's affinity 0-3
# irq        default_smp_affinity f; smp_affinity_list of /proc/irq/*: 0-3 x29, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: environment-man x10, process_api x1, vsock-console x1, idle-reclaim x1, sbx-telemetry-s x1, 6 x1, sh x1, claude x1, mi-scavenger x1, Bun Pool 0 x1, Bun Pool 1 x1, Bun Pool 2 x1, Bun Pool 3 x1, HTTP Client x1, fs.watch x1, JSCWarmUp x1, JITWorker x1, sleep x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.18 2.13 2.06 1/103 13919
# codec processes pinned to CPU 1 (taskset); nothing else of this session runs meanwhile
# committed: /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/codec_suite-3211c86b37bffe91 (sha256 c85a94d23aa4828c) loads /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (sha256 577b9b2e83079ce9)
# backward:  /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-bwd/ffi/poc/rust/target/release/deps/codec_suite-7e804e611118a090 (sha256 5871bc09622312e1) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-bwd/ffi/poc/rust/target/release/deps/libak_core.so (sha256 5216de8f7a664be7)
# codec settings: AK_ONLY=P1,P2,P3,P4,P5,P6 AK_CASE_ARMS=core-ffi,core-native AK_CASE_DIRS=encode AK_CASE_END=reused-buffer,transport-ready-tonic AK_CASE_INPUT=hot AK_SAMPLES=10 AK_WARMUP_MS=100 AK_MEASURE_MS=250; launches 3 per variant, alternated
codec committed launch 1: 75 s; # precheck: 660 checks, 0 failures, 21 inputs, 168 cases; # narrowed: arms [core-ffi,core-native] dirs [encode] end states [reused-buffer,transport-ready-tonic] inputs [hot]: 168 cases kept
codec backward launch 1: 76 s; # precheck: 660 checks, 0 failures, 21 inputs, 168 cases; # narrowed: arms [core-ffi,core-native] dirs [encode] end states [reused-buffer,transport-ready-tonic] inputs [hot]: 168 cases kept
codec committed launch 2: 76 s; # precheck: 660 checks, 0 failures, 21 inputs, 168 cases; # narrowed: arms [core-ffi,core-native] dirs [encode] end states [reused-buffer,transport-ready-tonic] inputs [hot]: 168 cases kept
codec backward launch 2: 75 s; # precheck: 660 checks, 0 failures, 21 inputs, 168 cases; # narrowed: arms [core-ffi,core-native] dirs [encode] end states [reused-buffer,transport-ready-tonic] inputs [hot]: 168 cases kept
codec committed launch 3: 76 s; # precheck: 660 checks, 0 failures, 21 inputs, 168 cases; # narrowed: arms [core-ffi,core-native] dirs [encode] end states [reused-buffer,transport-ready-tonic] inputs [hot]: 168 cases kept
codec backward launch 3: 75 s; # precheck: 660 checks, 0 failures, 21 inputs, 168 cases; # narrowed: arms [core-ffi,core-native] dirs [encode] end states [reused-buffer,transport-ready-tonic] inputs [hot]: 168 cases kept
# codec benchmark wall: 453 s
# rpc: serve.sh rpc_server pid 14317 on CPU 3, TCP 127.0.0.1:45155; probes on CPU 1-2
rpc committed process 1: rc 0, 32 rows; probe loads /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so
rpc backward process 1: rc 0, 32 rows; probe loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-bwd/ffi/poc/rust/target/release/deps/libak_core.so
rpc committed process 2: rc 0, 32 rows; probe loads /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so
rpc backward process 2: rc 0, 32 rows; probe loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-bwd/ffi/poc/rust/target/release/deps/libak_core.so
# rpc benchmark wall: 7 s
```

Absolute ns of process CPU per encode, per variant; no ratio is formed here. Each cell: the median of the
per-launch medians (10 criterion samples per launch), [min - max] of the launch medians, then p10-p90 of
every sample of every launch. `committed` = this branch's core and binding; `backward` = the patched core
with the binding that delivers every repeated field last to first. core-native (the forward ak_rt::Enc,
unchanged) is the in-process control row of each process. End states (requirement 11): reused-buffer, and
transport-ready-tonic (core-ffi: ak_enc_take_owned's moved buffer; core-native: Enc::take). Input: hot.

## P1.1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 293 ns [282 ns - 300 ns] (p10-p90 282 ns - 302 ns, 3 launches) | 302 ns [300 ns - 303 ns] (p10-p90 299 ns - 310 ns, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 432 ns [425 ns - 439 ns] (p10-p90 424 ns - 448 ns, 3 launches) | 425 ns [423 ns - 425 ns] (p10-p90 419 ns - 430 ns, 3 launches) |
| core-ffi | retain | reused-buffer | 288 ns [287 ns - 290 ns] (p10-p90 287 ns - 292 ns, 3 launches) | 317 ns [315 ns - 326 ns] (p10-p90 313 ns - 338 ns, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 435 ns [429 ns - 453 ns] (p10-p90 428 ns - 456 ns, 3 launches) | 429 ns [426 ns - 433 ns] (p10-p90 423 ns - 436 ns, 3 launches) |
| core-native | drop | reused-buffer | 203 ns [202 ns - 203 ns] (p10-p90 198 ns - 217 ns, 3 launches) | 199 ns [196 ns - 206 ns] (p10-p90 195 ns - 208 ns, 3 launches) |
| core-native | drop | transport-ready-tonic | 255 ns [254 ns - 256 ns] (p10-p90 252 ns - 271 ns, 3 launches) | 257 ns [253 ns - 258 ns] (p10-p90 251 ns - 262 ns, 3 launches) |
| core-native | retain | reused-buffer | 195 ns [194 ns - 200 ns] (p10-p90 193 ns - 200 ns, 3 launches) | 204 ns [203 ns - 204 ns] (p10-p90 203 ns - 207 ns, 3 launches) |
| core-native | retain | transport-ready-tonic | 251 ns [249 ns - 253 ns] (p10-p90 248 ns - 255 ns, 3 launches) | 255 ns [252 ns - 257 ns] (p10-p90 252 ns - 260 ns, 3 launches) |

## P1.2

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 85.7 us [85.5 us - 86.3 us] (p10-p90 85.1 us - 87.1 us, 3 launches) | 89.2 us [88.0 us - 89.4 us] (p10-p90 87.7 us - 90.3 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 86.8 us [86.6 us - 87.8 us] (p10-p90 85.8 us - 89.7 us, 3 launches) | 93.0 us [89.1 us - 94.4 us] (p10-p90 88.8 us - 95.4 us, 3 launches) |
| core-ffi | retain | reused-buffer | 88.5 us [87.8 us - 88.9 us] (p10-p90 87.7 us - 90.0 us, 3 launches) | 93.3 us [91.3 us - 93.9 us] (p10-p90 91.0 us - 95.3 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 90.0 us [88.7 us - 90.2 us] (p10-p90 88.6 us - 92.5 us, 3 launches) | 91.8 us [91.4 us - 95.5 us] (p10-p90 91.1 us - 100.9 us, 3 launches) |
| core-native | drop | reused-buffer | 56.8 us [55.7 us - 58.5 us] (p10-p90 55.6 us - 60.2 us, 3 launches) | 54.5 us [54.5 us - 54.9 us] (p10-p90 54.3 us - 57.2 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 56.6 us [56.1 us - 58.2 us] (p10-p90 55.9 us - 59.7 us, 3 launches) | 54.6 us [54.5 us - 55.4 us] (p10-p90 54.3 us - 55.6 us, 3 launches) |
| core-native | retain | reused-buffer | 58.5 us [58.2 us - 58.7 us] (p10-p90 58.0 us - 59.6 us, 3 launches) | 59.4 us [58.8 us - 59.5 us] (p10-p90 58.7 us - 60.5 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 60.4 us [58.5 us - 60.7 us] (p10-p90 58.4 us - 61.5 us, 3 launches) | 60.0 us [59.2 us - 62.7 us] (p10-p90 59.1 us - 62.9 us, 3 launches) |

## P1.2/latin1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 87.3 us [87.0 us - 87.9 us] (p10-p90 85.8 us - 90.5 us, 3 launches) | 86.3 us [86.3 us - 86.7 us] (p10-p90 86.0 us - 86.9 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 87.8 us [86.4 us - 88.2 us] (p10-p90 86.4 us - 89.1 us, 3 launches) | 89.9 us [87.9 us - 91.8 us] (p10-p90 87.8 us - 94.0 us, 3 launches) |
| core-ffi | retain | reused-buffer | 88.4 us [88.1 us - 88.9 us] (p10-p90 88.0 us - 89.5 us, 3 launches) | 88.0 us [87.4 us - 88.7 us] (p10-p90 87.1 us - 89.7 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 91.7 us [89.4 us - 95.4 us] (p10-p90 89.1 us - 96.5 us, 3 launches) | 89.7 us [88.9 us - 89.9 us] (p10-p90 88.4 us - 91.5 us, 3 launches) |
| core-native | drop | reused-buffer | 54.1 us [53.5 us - 54.2 us] (p10-p90 53.3 us - 57.0 us, 3 launches) | 53.0 us [52.8 us - 54.0 us] (p10-p90 52.6 us - 54.2 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 55.2 us [54.2 us - 56.0 us] (p10-p90 54.0 us - 57.0 us, 3 launches) | 53.6 us [52.7 us - 55.4 us] (p10-p90 52.5 us - 55.8 us, 3 launches) |
| core-native | retain | reused-buffer | 57.4 us [56.9 us - 57.7 us] (p10-p90 56.4 us - 60.4 us, 3 launches) | 57.0 us [56.9 us - 57.0 us] (p10-p90 56.5 us - 57.9 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 61.7 us [58.0 us - 62.6 us] (p10-p90 58.0 us - 63.8 us, 3 launches) | 57.7 us [57.0 us - 58.7 us] (p10-p90 56.8 us - 59.4 us, 3 launches) |

## P1.2/wide

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 91.9 us [88.1 us - 94.5 us] (p10-p90 87.9 us - 95.8 us, 3 launches) | 87.9 us [87.6 us - 88.8 us] (p10-p90 87.0 us - 89.7 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 89.7 us [89.5 us - 92.4 us] (p10-p90 89.3 us - 92.6 us, 3 launches) | 88.4 us [88.1 us - 93.4 us] (p10-p90 87.9 us - 102.1 us, 3 launches) |
| core-ffi | retain | reused-buffer | 92.8 us [91.2 us - 96.4 us] (p10-p90 90.9 us - 98.4 us, 3 launches) | 88.7 us [87.7 us - 90.9 us] (p10-p90 87.4 us - 93.0 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 97.2 us [92.4 us - 99.2 us] (p10-p90 92.3 us - 100.1 us, 3 launches) | 91.4 us [91.2 us - 92.2 us] (p10-p90 89.6 us - 93.9 us, 3 launches) |
| core-native | drop | reused-buffer | 58.4 us [57.3 us - 59.2 us] (p10-p90 57.1 us - 60.0 us, 3 launches) | 56.5 us [55.7 us - 57.4 us] (p10-p90 55.6 us - 57.7 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 57.9 us [57.1 us - 58.6 us] (p10-p90 57.1 us - 59.3 us, 3 launches) | 57.8 us [57.5 us - 58.1 us] (p10-p90 57.1 us - 59.0 us, 3 launches) |
| core-native | retain | reused-buffer | 59.9 us [58.5 us - 62.2 us] (p10-p90 58.5 us - 62.3 us, 3 launches) | 58.8 us [58.5 us - 61.7 us] (p10-p90 58.5 us - 62.2 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 65.9 us [60.3 us - 67.1 us] (p10-p90 60.3 us - 68.3 us, 3 launches) | 60.7 us [60.4 us - 61.0 us] (p10-p90 60.0 us - 61.4 us, 3 launches) |

## P1.3

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 3.97 us [3.96 us - 4.06 us] (p10-p90 3.93 us - 4.17 us, 3 launches) | 3.77 us [3.65 us - 3.78 us] (p10-p90 3.65 us - 3.82 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 4.07 us [4.03 us - 4.13 us] (p10-p90 4.02 us - 4.25 us, 3 launches) | 3.79 us [3.78 us - 3.95 us] (p10-p90 3.76 us - 4.02 us, 3 launches) |
| core-ffi | retain | reused-buffer | 4.02 us [3.94 us - 4.10 us] (p10-p90 3.93 us - 4.16 us, 3 launches) | 3.64 us [3.58 us - 3.66 us] (p10-p90 3.54 us - 3.70 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 4.11 us [4.08 us - 4.12 us] (p10-p90 4.07 us - 4.17 us, 3 launches) | 3.70 us [3.69 us - 3.81 us] (p10-p90 3.68 us - 3.84 us, 3 launches) |
| core-native | drop | reused-buffer | 2.28 us [2.24 us - 2.31 us] (p10-p90 2.22 us - 2.49 us, 3 launches) | 2.34 us [2.32 us - 2.35 us] (p10-p90 2.30 us - 2.37 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 2.29 us [2.29 us - 2.33 us] (p10-p90 2.27 us - 2.35 us, 3 launches) | 2.31 us [2.29 us - 2.53 us] (p10-p90 2.27 us - 2.72 us, 3 launches) |
| core-native | retain | reused-buffer | 2.24 us [2.23 us - 2.28 us] (p10-p90 2.22 us - 2.29 us, 3 launches) | 2.44 us [2.40 us - 2.48 us] (p10-p90 2.39 us - 2.49 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 2.44 us [2.39 us - 2.46 us] (p10-p90 2.38 us - 2.47 us, 3 launches) | 2.56 us [2.51 us - 2.71 us] (p10-p90 2.49 us - 2.72 us, 3 launches) |

## P2.1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 479 ns [477 ns - 502 ns] (p10-p90 468 ns - 505 ns, 3 launches) | 456 ns [449 ns - 458 ns] (p10-p90 447 ns - 461 ns, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 631 ns [616 ns - 634 ns] (p10-p90 611 ns - 641 ns, 3 launches) | 577 ns [574 ns - 613 ns] (p10-p90 571 ns - 624 ns, 3 launches) |
| core-ffi | retain | reused-buffer | 489 ns [485 ns - 524 ns] (p10-p90 483 ns - 524 ns, 3 launches) | 468 ns [467 ns - 471 ns] (p10-p90 465 ns - 479 ns, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 652 ns [643 ns - 652 ns] (p10-p90 640 ns - 656 ns, 3 launches) | 576 ns [571 ns - 641 ns] (p10-p90 571 ns - 660 ns, 3 launches) |
| core-native | drop | reused-buffer | 250 ns [249 ns - 251 ns] (p10-p90 247 ns - 255 ns, 3 launches) | 257 ns [256 ns - 259 ns] (p10-p90 254 ns - 262 ns, 3 launches) |
| core-native | drop | transport-ready-tonic | 296 ns [294 ns - 302 ns] (p10-p90 293 ns - 311 ns, 3 launches) | 302 ns [301 ns - 303 ns] (p10-p90 300 ns - 304 ns, 3 launches) |
| core-native | retain | reused-buffer | 263 ns [263 ns - 268 ns] (p10-p90 261 ns - 268 ns, 3 launches) | 266 ns [261 ns - 267 ns] (p10-p90 260 ns - 277 ns, 3 launches) |
| core-native | retain | transport-ready-tonic | 318 ns [314 ns - 325 ns] (p10-p90 312 ns - 335 ns, 3 launches) | 313 ns [308 ns - 315 ns] (p10-p90 308 ns - 322 ns, 3 launches) |

## P2.2

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 286.3 us [285.5 us - 286.5 us] (p10-p90 283.3 us - 294.8 us, 3 launches) | 301.3 us [299.2 us - 312.0 us] (p10-p90 297.5 us - 314.1 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 298.2 us [287.3 us - 301.9 us] (p10-p90 286.2 us - 311.3 us, 3 launches) | 297.8 us [295.5 us - 301.5 us] (p10-p90 295.1 us - 305.3 us, 3 launches) |
| core-ffi | retain | reused-buffer | 303.5 us [303.4 us - 304.3 us] (p10-p90 298.3 us - 307.7 us, 3 launches) | 297.9 us [293.8 us - 301.2 us] (p10-p90 293.2 us - 303.4 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 304.2 us [299.9 us - 310.9 us] (p10-p90 298.8 us - 313.0 us, 3 launches) | 301.7 us [298.6 us - 315.0 us] (p10-p90 296.2 us - 317.4 us, 3 launches) |
| core-native | drop | reused-buffer | 173.2 us [166.2 us - 180.8 us] (p10-p90 165.7 us - 183.8 us, 3 launches) | 170.1 us [169.1 us - 188.5 us] (p10-p90 168.4 us - 192.7 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 170.8 us [167.4 us - 176.1 us] (p10-p90 165.8 us - 180.7 us, 3 launches) | 178.0 us [169.1 us - 179.7 us] (p10-p90 169.0 us - 183.7 us, 3 launches) |
| core-native | retain | reused-buffer | 179.0 us [174.0 us - 179.3 us] (p10-p90 173.7 us - 182.2 us, 3 launches) | 172.3 us [171.5 us - 177.1 us] (p10-p90 171.0 us - 180.5 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 179.7 us [176.4 us - 184.6 us] (p10-p90 174.9 us - 187.2 us, 3 launches) | 175.3 us [173.6 us - 181.6 us] (p10-p90 173.0 us - 183.0 us, 3 launches) |

## P2.2/latin1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 299.1 us [294.8 us - 303.2 us] (p10-p90 291.6 us - 308.6 us, 3 launches) | 300.8 us [300.7 us - 308.8 us] (p10-p90 299.6 us - 310.3 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 296.7 us [295.6 us - 297.5 us] (p10-p90 294.5 us - 300.1 us, 3 launches) | 306.6 us [304.3 us - 317.0 us] (p10-p90 302.7 us - 327.6 us, 3 launches) |
| core-ffi | retain | reused-buffer | 308.9 us [304.1 us - 309.9 us] (p10-p90 302.2 us - 317.3 us, 3 launches) | 316.7 us [303.7 us - 321.3 us] (p10-p90 303.3 us - 333.1 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 308.0 us [307.5 us - 319.9 us] (p10-p90 306.1 us - 329.2 us, 3 launches) | 309.3 us [305.7 us - 309.5 us] (p10-p90 304.7 us - 316.4 us, 3 launches) |
| core-native | drop | reused-buffer | 196.9 us [196.4 us - 199.2 us] (p10-p90 195.5 us - 201.9 us, 3 launches) | 207.2 us [202.4 us - 220.6 us] (p10-p90 201.3 us - 231.7 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 202.4 us [200.2 us - 218.8 us] (p10-p90 198.1 us - 235.8 us, 3 launches) | 203.9 us [202.8 us - 210.2 us] (p10-p90 200.9 us - 212.6 us, 3 launches) |
| core-native | retain | reused-buffer | 204.3 us [201.1 us - 206.3 us] (p10-p90 199.2 us - 208.7 us, 3 launches) | 203.4 us [201.8 us - 210.4 us] (p10-p90 201.4 us - 213.3 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 202.3 us [201.3 us - 210.7 us] (p10-p90 200.0 us - 214.7 us, 3 launches) | 206.7 us [201.7 us - 209.3 us] (p10-p90 201.3 us - 213.5 us, 3 launches) |

## P2.2/wide

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 295.5 us [292.9 us - 311.3 us] (p10-p90 289.3 us - 318.4 us, 3 launches) | 306.7 us [305.1 us - 311.4 us] (p10-p90 302.9 us - 315.3 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 296.2 us [296.1 us - 306.6 us] (p10-p90 294.4 us - 310.7 us, 3 launches) | 301.5 us [299.3 us - 321.5 us] (p10-p90 298.6 us - 330.3 us, 3 launches) |
| core-ffi | retain | reused-buffer | 302.8 us [300.2 us - 303.2 us] (p10-p90 293.6 us - 320.6 us, 3 launches) | 315.0 us [312.7 us - 317.9 us] (p10-p90 307.7 us - 320.5 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 305.5 us [296.6 us - 314.7 us] (p10-p90 295.9 us - 322.3 us, 3 launches) | 322.1 us [309.7 us - 367.2 us] (p10-p90 308.7 us - 370.2 us, 3 launches) |
| core-native | drop | reused-buffer | 228.7 us [227.7 us - 233.2 us] (p10-p90 225.8 us - 235.5 us, 3 launches) | 233.1 us [225.5 us - 236.8 us] (p10-p90 224.7 us - 240.5 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 236.3 us [235.5 us - 241.0 us] (p10-p90 233.1 us - 248.9 us, 3 launches) | 232.5 us [230.3 us - 235.3 us] (p10-p90 229.6 us - 239.4 us, 3 launches) |
| core-native | retain | reused-buffer | 233.1 us [228.9 us - 234.2 us] (p10-p90 226.8 us - 235.8 us, 3 launches) | 231.4 us [230.8 us - 231.8 us] (p10-p90 228.9 us - 234.6 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 237.4 us [227.8 us - 238.9 us] (p10-p90 226.8 us - 242.2 us, 3 launches) | 233.8 us [225.7 us - 237.8 us] (p10-p90 225.7 us - 239.6 us, 3 launches) |

## P2.3

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 159.9 us [156.2 us - 166.3 us] (p10-p90 155.4 us - 166.9 us, 3 launches) | 160.5 us [158.1 us - 168.0 us] (p10-p90 157.2 us - 173.1 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 158.2 us [157.9 us - 163.0 us] (p10-p90 157.2 us - 164.0 us, 3 launches) | 163.2 us [162.2 us - 165.0 us] (p10-p90 162.1 us - 168.1 us, 3 launches) |
| core-ffi | retain | reused-buffer | 164.7 us [161.5 us - 167.8 us] (p10-p90 160.4 us - 170.9 us, 3 launches) | 161.9 us [161.9 us - 167.4 us] (p10-p90 159.5 us - 170.3 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 165.3 us [164.6 us - 169.7 us] (p10-p90 162.1 us - 175.1 us, 3 launches) | 169.4 us [165.3 us - 176.9 us] (p10-p90 163.3 us - 177.4 us, 3 launches) |
| core-native | drop | reused-buffer | 101.7 us [96.5 us - 101.8 us] (p10-p90 96.4 us - 105.6 us, 3 launches) | 95.4 us [94.0 us - 97.2 us] (p10-p90 93.9 us - 98.4 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 100.8 us [100.5 us - 103.7 us] (p10-p90 99.8 us - 104.6 us, 3 launches) | 102.9 us [101.3 us - 108.2 us] (p10-p90 100.7 us - 108.6 us, 3 launches) |
| core-native | retain | reused-buffer | 97.9 us [97.5 us - 98.1 us] (p10-p90 97.3 us - 98.4 us, 3 launches) | 100.3 us [98.5 us - 100.4 us] (p10-p90 98.0 us - 101.9 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 101.5 us [100.9 us - 104.0 us] (p10-p90 100.7 us - 104.6 us, 3 launches) | 100.6 us [99.6 us - 103.4 us] (p10-p90 99.2 us - 105.5 us, 3 launches) |

## P2.4

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 240.5 us [233.2 us - 248.1 us] (p10-p90 232.8 us - 250.0 us, 3 launches) | 217.0 us [214.9 us - 220.4 us] (p10-p90 214.2 us - 223.1 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 255.7 us [238.1 us - 263.5 us] (p10-p90 237.1 us - 267.4 us, 3 launches) | 223.2 us [222.7 us - 225.2 us] (p10-p90 220.7 us - 230.3 us, 3 launches) |
| core-ffi | retain | reused-buffer | 250.3 us [233.6 us - 260.1 us] (p10-p90 232.9 us - 263.8 us, 3 launches) | 220.5 us [214.9 us - 222.1 us] (p10-p90 214.8 us - 224.8 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 247.6 us [244.1 us - 252.4 us] (p10-p90 241.9 us - 255.2 us, 3 launches) | 220.0 us [219.5 us - 220.2 us] (p10-p90 217.7 us - 223.5 us, 3 launches) |
| core-native | drop | reused-buffer | 151.5 us [149.9 us - 152.5 us] (p10-p90 149.7 us - 156.2 us, 3 launches) | 152.1 us [151.7 us - 155.1 us] (p10-p90 149.8 us - 159.2 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 152.0 us [147.7 us - 163.2 us] (p10-p90 147.0 us - 163.7 us, 3 launches) | 154.7 us [151.3 us - 156.4 us] (p10-p90 151.2 us - 157.3 us, 3 launches) |
| core-native | retain | reused-buffer | 152.0 us [151.3 us - 154.3 us] (p10-p90 150.6 us - 156.2 us, 3 launches) | 148.9 us [148.1 us - 151.0 us] (p10-p90 147.8 us - 151.3 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 152.9 us [152.6 us - 163.7 us] (p10-p90 151.9 us - 165.2 us, 3 launches) | 149.7 us [148.4 us - 153.0 us] (p10-p90 147.2 us - 153.5 us, 3 launches) |

## P2.4/latin1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 324.4 us [321.3 us - 326.4 us] (p10-p90 318.6 us - 335.5 us, 3 launches) | 309.7 us [297.7 us - 315.4 us] (p10-p90 297.7 us - 320.7 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 331.2 us [329.8 us - 336.3 us] (p10-p90 319.4 us - 350.6 us, 3 launches) | 300.6 us [300.0 us - 306.9 us] (p10-p90 296.4 us - 312.8 us, 3 launches) |
| core-ffi | retain | reused-buffer | 324.7 us [314.1 us - 332.6 us] (p10-p90 314.0 us - 335.2 us, 3 launches) | 307.3 us [302.0 us - 311.0 us] (p10-p90 301.4 us - 314.8 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 328.0 us [321.9 us - 346.2 us] (p10-p90 321.4 us - 346.8 us, 3 launches) | 313.6 us [300.1 us - 317.4 us] (p10-p90 299.4 us - 322.0 us, 3 launches) |
| core-native | drop | reused-buffer | 252.6 us [251.8 us - 258.1 us] (p10-p90 250.8 us - 259.1 us, 3 launches) | 254.4 us [253.3 us - 256.5 us] (p10-p90 252.5 us - 257.4 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 252.5 us [252.3 us - 253.1 us] (p10-p90 251.8 us - 254.1 us, 3 launches) | 252.6 us [252.1 us - 253.0 us] (p10-p90 250.9 us - 256.9 us, 3 launches) |
| core-native | retain | reused-buffer | 252.8 us [245.9 us - 255.7 us] (p10-p90 244.3 us - 259.3 us, 3 launches) | 254.0 us [247.5 us - 255.5 us] (p10-p90 247.1 us - 256.6 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 253.8 us [252.9 us - 259.2 us] (p10-p90 251.9 us - 262.6 us, 3 launches) | 252.2 us [251.5 us - 259.8 us] (p10-p90 248.6 us - 260.2 us, 3 launches) |

## P2.4/wide

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 408.5 us [400.9 us - 409.2 us] (p10-p90 399.3 us - 416.2 us, 3 launches) | 386.8 us [380.6 us - 393.5 us] (p10-p90 373.7 us - 396.2 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 416.0 us [413.1 us - 423.1 us] (p10-p90 409.8 us - 424.9 us, 3 launches) | 404.4 us [386.5 us - 407.1 us] (p10-p90 385.7 us - 411.4 us, 3 launches) |
| core-ffi | retain | reused-buffer | 414.9 us [401.2 us - 417.6 us] (p10-p90 400.6 us - 425.3 us, 3 launches) | 389.7 us [389.2 us - 401.0 us] (p10-p90 386.3 us - 403.2 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 414.8 us [410.5 us - 421.4 us] (p10-p90 409.9 us - 424.0 us, 3 launches) | 399.5 us [396.7 us - 405.9 us] (p10-p90 393.7 us - 410.2 us, 3 launches) |
| core-native | drop | reused-buffer | 337.8 us [337.2 us - 345.3 us] (p10-p90 336.7 us - 353.0 us, 3 launches) | 358.6 us [351.7 us - 365.5 us] (p10-p90 350.8 us - 369.4 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 339.7 us [337.0 us - 345.7 us] (p10-p90 335.8 us - 348.7 us, 3 launches) | 342.5 us [341.4 us - 343.8 us] (p10-p90 339.4 us - 345.9 us, 3 launches) |
| core-native | retain | reused-buffer | 342.0 us [341.9 us - 345.7 us] (p10-p90 340.2 us - 347.3 us, 3 launches) | 333.2 us [330.7 us - 338.5 us] (p10-p90 317.9 us - 347.8 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 330.8 us [328.3 us - 346.0 us] (p10-p90 326.7 us - 347.2 us, 3 launches) | 340.1 us [337.7 us - 341.5 us] (p10-p90 337.3 us - 343.7 us, 3 launches) |

## P2.5

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 8.79 us [8.65 us - 9.23 us] (p10-p90 8.61 us - 9.48 us, 3 launches) | 8.63 us [8.59 us - 8.95 us] (p10-p90 8.56 us - 8.96 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 8.89 us [8.79 us - 9.18 us] (p10-p90 8.76 us - 9.26 us, 3 launches) | 8.98 us [8.95 us - 9.10 us] (p10-p90 8.89 us - 9.25 us, 3 launches) |
| core-ffi | retain | reused-buffer | 9.01 us [8.96 us - 9.11 us] (p10-p90 8.85 us - 9.31 us, 3 launches) | 8.82 us [8.56 us - 8.85 us] (p10-p90 8.55 us - 8.99 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 9.18 us [9.02 us - 9.32 us] (p10-p90 9.00 us - 9.37 us, 3 launches) | 8.87 us [8.83 us - 8.87 us] (p10-p90 8.80 us - 8.99 us, 3 launches) |
| core-native | drop | reused-buffer | 4.73 us [4.70 us - 4.79 us] (p10-p90 4.65 us - 4.85 us, 3 launches) | 4.70 us [4.67 us - 4.75 us] (p10-p90 4.64 us - 4.84 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 4.86 us [4.76 us - 4.87 us] (p10-p90 4.74 us - 4.97 us, 3 launches) | 4.80 us [4.78 us - 4.88 us] (p10-p90 4.77 us - 4.94 us, 3 launches) |
| core-native | retain | reused-buffer | 4.94 us [4.80 us - 4.95 us] (p10-p90 4.79 us - 5.02 us, 3 launches) | 4.84 us [4.76 us - 4.90 us] (p10-p90 4.75 us - 5.19 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 5.00 us [4.95 us - 5.03 us] (p10-p90 4.91 us - 5.16 us, 3 launches) | 4.90 us [4.85 us - 5.03 us] (p10-p90 4.84 us - 5.11 us, 3 launches) |

## P3.1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 6.20 us [6.14 us - 6.26 us] (p10-p90 6.13 us - 6.28 us, 3 launches) | 5.92 us [5.90 us - 6.23 us] (p10-p90 5.88 us - 6.42 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 6.33 us [6.31 us - 6.44 us] (p10-p90 6.29 us - 6.52 us, 3 launches) | 6.12 us [6.11 us - 6.22 us] (p10-p90 6.08 us - 6.30 us, 3 launches) |
| core-ffi | retain | reused-buffer | 6.39 us [6.35 us - 6.49 us] (p10-p90 6.32 us - 6.72 us, 3 launches) | 6.27 us [6.26 us - 6.27 us] (p10-p90 6.23 us - 6.40 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 6.57 us [6.46 us - 6.72 us] (p10-p90 6.45 us - 6.76 us, 3 launches) | 6.58 us [6.53 us - 6.61 us] (p10-p90 6.46 us - 6.75 us, 3 launches) |
| core-native | drop | reused-buffer | 3.68 us [3.65 us - 3.71 us] (p10-p90 3.63 us - 3.74 us, 3 launches) | 4.08 us [4.06 us - 4.10 us] (p10-p90 4.03 us - 4.14 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 3.84 us [3.77 us - 4.08 us] (p10-p90 3.72 us - 4.29 us, 3 launches) | 4.14 us [4.10 us - 4.14 us] (p10-p90 4.09 us - 4.18 us, 3 launches) |
| core-native | retain | reused-buffer | 3.76 us [3.75 us - 3.77 us] (p10-p90 3.73 us - 4.06 us, 3 launches) | 3.98 us [3.97 us - 4.00 us] (p10-p90 3.93 us - 4.06 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 3.83 us [3.82 us - 3.89 us] (p10-p90 3.81 us - 3.91 us, 3 launches) | 4.02 us [3.95 us - 4.12 us] (p10-p90 3.95 us - 4.22 us, 3 launches) |

## P4.1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 43.4 us [42.2 us - 45.4 us] (p10-p90 41.9 us - 45.9 us, 3 launches) | 44.3 us [44.1 us - 47.6 us] (p10-p90 44.0 us - 47.9 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 44.8 us [43.2 us - 45.0 us] (p10-p90 43.1 us - 48.8 us, 3 launches) | 45.4 us [44.0 us - 46.0 us] (p10-p90 43.8 us - 46.3 us, 3 launches) |
| core-ffi | retain | reused-buffer | 45.6 us [44.4 us - 45.8 us] (p10-p90 44.1 us - 46.4 us, 3 launches) | 46.1 us [45.5 us - 46.7 us] (p10-p90 45.2 us - 48.0 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 46.6 us [45.4 us - 46.7 us] (p10-p90 45.1 us - 48.2 us, 3 launches) | 45.3 us [45.3 us - 46.2 us] (p10-p90 44.6 us - 47.4 us, 3 launches) |
| core-native | drop | reused-buffer | 23.4 us [22.9 us - 24.5 us] (p10-p90 22.9 us - 24.8 us, 3 launches) | 25.1 us [24.0 us - 25.2 us] (p10-p90 23.9 us - 25.8 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 23.5 us [23.3 us - 24.0 us] (p10-p90 23.2 us - 24.4 us, 3 launches) | 24.9 us [24.8 us - 25.3 us] (p10-p90 24.6 us - 25.8 us, 3 launches) |
| core-native | retain | reused-buffer | 24.6 us [24.1 us - 25.8 us] (p10-p90 24.0 us - 28.3 us, 3 launches) | 23.1 us [23.0 us - 23.5 us] (p10-p90 22.8 us - 23.8 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 25.3 us [24.4 us - 26.2 us] (p10-p90 24.3 us - 27.6 us, 3 launches) | 23.5 us [23.4 us - 24.0 us] (p10-p90 23.3 us - 24.7 us, 3 launches) |

## P5.1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 34 ns [34 ns - 35 ns] (p10-p90 34 ns - 35 ns, 3 launches) | 33 ns [33 ns - 33 ns] (p10-p90 33 ns - 33 ns, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 176 ns [175 ns - 176 ns] (p10-p90 173 ns - 178 ns, 3 launches) | 153 ns [150 ns - 153 ns] (p10-p90 150 ns - 155 ns, 3 launches) |
| core-ffi | retain | reused-buffer | 36 ns [36 ns - 37 ns] (p10-p90 35 ns - 37 ns, 3 launches) | 34 ns [34 ns - 35 ns] (p10-p90 34 ns - 36 ns, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 177 ns [175 ns - 178 ns] (p10-p90 175 ns - 181 ns, 3 launches) | 152 ns [151 ns - 153 ns] (p10-p90 151 ns - 153 ns, 3 launches) |
| core-native | drop | reused-buffer | 19 ns [18 ns - 19 ns] (p10-p90 18 ns - 19 ns, 3 launches) | 19 ns [18 ns - 19 ns] (p10-p90 18 ns - 19 ns, 3 launches) |
| core-native | drop | transport-ready-tonic | 102 ns [102 ns - 103 ns] (p10-p90 101 ns - 104 ns, 3 launches) | 103 ns [102 ns - 103 ns] (p10-p90 101 ns - 103 ns, 3 launches) |
| core-native | retain | reused-buffer | 19 ns [18 ns - 19 ns] (p10-p90 18 ns - 19 ns, 3 launches) | 19 ns [19 ns - 19 ns] (p10-p90 19 ns - 19 ns, 3 launches) |
| core-native | retain | transport-ready-tonic | 105 ns [102 ns - 105 ns] (p10-p90 101 ns - 106 ns, 3 launches) | 101 ns [101 ns - 102 ns] (p10-p90 100 ns - 103 ns, 3 launches) |

## P5.2

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 1.84 us [1.81 us - 1.89 us] (p10-p90 1.80 us - 1.90 us, 3 launches) | 1.77 us [1.76 us - 1.87 us] (p10-p90 1.76 us - 1.87 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 1.97 us [1.95 us - 1.98 us] (p10-p90 1.95 us - 1.99 us, 3 launches) | 1.91 us [1.91 us - 1.93 us] (p10-p90 1.90 us - 1.95 us, 3 launches) |
| core-ffi | retain | reused-buffer | 1.81 us [1.79 us - 1.85 us] (p10-p90 1.79 us - 1.87 us, 3 launches) | 1.77 us [1.76 us - 1.83 us] (p10-p90 1.76 us - 1.84 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 1.95 us [1.93 us - 1.96 us] (p10-p90 1.93 us - 1.97 us, 3 launches) | 1.93 us [1.90 us - 1.94 us] (p10-p90 1.90 us - 1.95 us, 3 launches) |
| core-native | drop | reused-buffer | 1.76 us [1.74 us - 1.77 us] (p10-p90 1.74 us - 1.78 us, 3 launches) | 1.77 us [1.76 us - 1.77 us] (p10-p90 1.75 us - 1.79 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 1.86 us [1.86 us - 1.89 us] (p10-p90 1.86 us - 1.90 us, 3 launches) | 1.95 us [1.92 us - 1.97 us] (p10-p90 1.91 us - 2.00 us, 3 launches) |
| core-native | retain | reused-buffer | 1.74 us [1.73 us - 1.76 us] (p10-p90 1.72 us - 1.77 us, 3 launches) | 1.76 us [1.74 us - 1.80 us] (p10-p90 1.74 us - 1.81 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 1.83 us [1.83 us - 1.85 us] (p10-p90 1.83 us - 1.86 us, 3 launches) | 1.83 us [1.83 us - 1.83 us] (p10-p90 1.82 us - 1.84 us, 3 launches) |

## P5.3

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 42.5 us [41.4 us - 44.4 us] (p10-p90 40.5 us - 45.1 us, 3 launches) | 42.7 us [39.2 us - 44.6 us] (p10-p90 39.1 us - 45.3 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 57.6 us [54.5 us - 60.1 us] (p10-p90 53.6 us - 61.1 us, 3 launches) | 55.6 us [54.5 us - 58.4 us] (p10-p90 53.2 us - 59.1 us, 3 launches) |
| core-ffi | retain | reused-buffer | 42.2 us [38.8 us - 42.8 us] (p10-p90 38.4 us - 43.2 us, 3 launches) | 39.3 us [38.3 us - 40.3 us] (p10-p90 38.2 us - 42.0 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 55.1 us [54.9 us - 55.9 us] (p10-p90 54.5 us - 56.9 us, 3 launches) | 56.0 us [55.3 us - 58.5 us] (p10-p90 55.0 us - 58.8 us, 3 launches) |
| core-native | drop | reused-buffer | 44.0 us [41.4 us - 45.2 us] (p10-p90 41.2 us - 46.2 us, 3 launches) | 41.3 us [39.7 us - 42.1 us] (p10-p90 39.6 us - 42.3 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 55.5 us [54.0 us - 56.2 us] (p10-p90 53.2 us - 57.6 us, 3 launches) | 55.0 us [54.7 us - 57.3 us] (p10-p90 53.6 us - 57.9 us, 3 launches) |
| core-native | retain | reused-buffer | 42.1 us [40.8 us - 44.4 us] (p10-p90 40.8 us - 44.9 us, 3 launches) | 40.8 us [39.1 us - 41.1 us] (p10-p90 39.1 us - 41.5 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 54.4 us [51.3 us - 56.3 us] (p10-p90 51.1 us - 57.3 us, 3 launches) | 53.9 us [51.6 us - 54.4 us] (p10-p90 51.1 us - 55.0 us, 3 launches) |

## P5.4

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 314.1 us [311.5 us - 315.0 us] (p10-p90 308.6 us - 321.5 us, 3 launches) | 316.0 us [305.3 us - 322.4 us] (p10-p90 305.2 us - 326.9 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 321.0 us [309.0 us - 332.5 us] (p10-p90 308.8 us - 333.5 us, 3 launches) | 318.1 us [305.0 us - 319.6 us] (p10-p90 304.7 us - 320.2 us, 3 launches) |
| core-ffi | retain | reused-buffer | 314.9 us [305.8 us - 326.4 us] (p10-p90 301.2 us - 328.6 us, 3 launches) | 314.4 us [309.8 us - 322.6 us] (p10-p90 309.2 us - 323.2 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 323.3 us [309.4 us - 328.2 us] (p10-p90 308.9 us - 328.5 us, 3 launches) | 316.8 us [311.9 us - 321.6 us] (p10-p90 310.3 us - 322.4 us, 3 launches) |
| core-native | drop | reused-buffer | 312.9 us [296.6 us - 322.2 us] (p10-p90 296.1 us - 326.1 us, 3 launches) | 321.5 us [316.8 us - 324.1 us] (p10-p90 314.9 us - 325.3 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 319.3 us [317.7 us - 322.6 us] (p10-p90 316.7 us - 325.2 us, 3 launches) | 321.0 us [319.9 us - 322.2 us] (p10-p90 317.6 us - 324.0 us, 3 launches) |
| core-native | retain | reused-buffer | 310.0 us [289.9 us - 329.8 us] (p10-p90 289.7 us - 331.8 us, 3 launches) | 320.3 us [319.2 us - 329.6 us] (p10-p90 317.5 us - 335.9 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 318.2 us [306.9 us - 322.6 us] (p10-p90 306.6 us - 323.9 us, 3 launches) | 322.8 us [314.3 us - 326.6 us] (p10-p90 313.4 us - 327.6 us, 3 launches) |

## P6.1

| arm | mode | end state | committed | backward |
|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 61.3 us [60.7 us - 62.6 us] (p10-p90 60.5 us - 63.7 us, 3 launches) | 75.5 us [75.2 us - 75.5 us] (p10-p90 74.7 us - 76.1 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 62.2 us [61.5 us - 63.0 us] (p10-p90 61.3 us - 63.6 us, 3 launches) | 76.1 us [75.6 us - 76.5 us] (p10-p90 75.5 us - 77.1 us, 3 launches) |
| core-ffi | retain | reused-buffer | 62.5 us [62.1 us - 62.9 us] (p10-p90 61.2 us - 64.4 us, 3 launches) | 77.3 us [76.5 us - 77.8 us] (p10-p90 76.2 us - 78.7 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 62.3 us [61.8 us - 65.4 us] (p10-p90 61.6 us - 65.7 us, 3 launches) | 78.3 us [76.7 us - 79.5 us] (p10-p90 76.3 us - 80.3 us, 3 launches) |
| core-native | drop | reused-buffer | 64.4 us [64.4 us - 66.9 us] (p10-p90 64.1 us - 67.2 us, 3 launches) | 56.6 us [56.1 us - 58.1 us] (p10-p90 56.0 us - 58.9 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 66.0 us [64.9 us - 67.5 us] (p10-p90 64.7 us - 67.7 us, 3 launches) | 57.4 us [56.7 us - 60.0 us] (p10-p90 56.5 us - 63.4 us, 3 launches) |
| core-native | retain | reused-buffer | 68.6 us [67.1 us - 70.5 us] (p10-p90 67.1 us - 70.9 us, 3 launches) | 61.1 us [61.1 us - 62.1 us] (p10-p90 60.5 us - 63.2 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 69.5 us [67.6 us - 70.0 us] (p10-p90 67.5 us - 71.2 us, 3 launches) | 61.9 us [61.3 us - 62.1 us] (p10-p90 61.0 us - 62.9 us, 3 launches) |

## RPC: directions d (16 MiB in 2 MiB chunks) and c (P5.4), k = 1, TCP 127.0.0.1, pinned configuration

Client process CPU per call (stream_probe `cpu_ns`, every call checked: status and the server's byte count),
median over every round of both processes per variant, [min - max] of the round values. Cell A (tonic +
prost, no core in its path) is the in-process control.

| cell | workload | committed | backward |
|---|---|---|---|
| A | d/16MiB | 10.65 ms [9.09 ms - 11.38 ms] (8 rounds) | 10.70 ms [9.14 ms - 11.92 ms] (8 rounds) |
| A | c/P5.4 | 2.55 ms [2.44 ms - 2.74 ms] (8 rounds) | 2.63 ms [2.35 ms - 2.96 ms] (8 rounds) |
| C-retain | d/16MiB | 13.15 ms [12.16 ms - 13.79 ms] (8 rounds) | 12.65 ms [11.21 ms - 14.17 ms] (8 rounds) |
| C-retain | c/P5.4 | 3.34 ms [3.16 ms - 4.20 ms] (8 rounds) | 3.28 ms [3.13 ms - 3.52 ms] (8 rounds) |
| Cf-retain | d/16MiB | 11.04 ms [9.85 ms - 12.77 ms] (8 rounds) | 11.63 ms [9.68 ms - 12.39 ms] (8 rounds) |
| Cf-retain | c/P5.4 | 2.96 ms [2.68 ms - 3.84 ms] (8 rounds) | 3.06 ms [2.77 ms - 3.27 ms] (8 rounds) |
| Df-retain | d/16MiB | 10.81 ms [9.68 ms - 16.97 ms] (8 rounds) | 10.81 ms [9.85 ms - 11.29 ms] (8 rounds) |
| Df-retain | c/P5.4 | 2.78 ms [2.52 ms - 3.05 ms] (8 rounds) | 2.84 ms [2.60 ms - 3.15 ms] (8 rounds) |

## Element calls per payload (counting build of the backward core, one warm encode)

```
# bwd_check: 78 passed, 0 failed
# per payload, one warm encode (the binding's loops: every field delivered last to first):
# elem_calls = element entry calls (ak_elem*/ak_elemu*/ak_blob_run/ak_run_*), each now in reverse order;
# items = elements they carried; multi = calls with n > 1 (array walked last to first by the codec);
# tc_moves/bytes = transcoder outputs moved after commit; grows/bytes = buffer grows (0 warm);
# core_fwd = the core-counted forward crossings (ak_enc_counters): elem_calls must equal core_fwd - 1 (the ak_encode_* call)
payload  mode      calls    items    multi tc_moves  tc_bytes  grows grow_bytes  core_fwd
P1.1     drop          1        4        1        0         0      0          0        2 ok
P1.1     retain        1        4        1        0         0      0          0        2 ok
P1.2     drop          7     1000        7        0         0      0          0        8 ok
P1.2     retain        9     1000        9        0         0      0          0       10 ok
P1.3     drop          2      300        2        0         0      0          0        3 ok
P1.3     retain        3      300        3        0         0      0          0        4 ok
P2.1     drop          6       17        5        0         0      0          0        7 ok
P2.1     retain        6       17        5        0         0      0          0        7 ok
P2.2     drop       2510     8500     2510        0         0      0          0     2511 ok
P2.2     retain     2516     8500     2516        0         0      0          0     2517 ok
P2.3     drop        628    15625      628        0         0      0          0      629 ok
P2.3     retain      629    15625      629        0         0      0          0      630 ok
P2.4     drop        402    24880      402        0         0      0          0      403 ok
P2.4     retain      403    24880      403        0         0      0          0      404 ok
P2.5     drop        101      340      101        0         0      0          0      102 ok
P2.5     retain      101      340      101        0         0      0          0      102 ok
P3.1     drop          1      200        1        0         0      0          0        2 ok
P3.1     retain        2      200        2        0         0      0          0        3 ok
P4.1     drop        202     1000      202        0         0      0          0      203 ok
P4.1     retain      203     1000      203        0         0      0          0      204 ok
P5.1     drop          0        0        0        0         0      0          0        1 ok
P5.1     retain        0        0        0        0         0      0          0        1 ok
P5.2     drop          0        0        0        0         0      0          0        1 ok
P5.2     retain        0        0        0        0         0      0          0        1 ok
P5.3     drop          0        0        0        0         0      0          0        1 ok
P5.3     retain        0        0        0        0         0      0          0        1 ok
P5.4     drop          0        0        0        0         0      0          0        1 ok
P5.4     retain        0        0        0        0         0      0          0        1 ok
P6.1     drop       1001    30200     1001        0         0      0          0     1002 ok
P6.1     retain     1001    30200     1001        0         0      0          0     1002 ok
```
