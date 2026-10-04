# D20: decode-read rows, before / after (all bits 0) / skipall (every string bit set)

CONTAINER INSTRUMENTATION, not campaign results. ns of process CPU per decode-read;
cell = median of the per-process medians [min - max]; ratio = per launch, median [min - max].

```
# D20 utf8_skip decode measurement, 2026-10-04T14:07:22Z. CONTAINER INSTRUMENTATION, not a campaign result.
# commit 4666f106dcd495112afa6309d420fef82298097c
# rustc 1.94.1 (e408947bf 2026-03-25); nproc 4
# cpu        Intel(R) Xeon(R) Processor @ 2.80GHz; kernel 6.18.44-fc-v64; smt notsupported (active 0); no_turbo n/a; governor cpu1 n/a; scaling min/max cpu1 n/a/n/a kHz
# isolation  cmdline: isolated='' nohz_full='n/a'
# cgroups    cpuset.cpus.effective:; this driver's cgroup /: n/a; root cpuset.cpus.isolated='n/a'; this driver's affinity 0-3
# irq        default_smp_affinity f; smp_affinity_list of /proc/irq/*: 0-3 x29, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: environment-man x11, HeapHelper x3, process_api x1, vsock-console x1, idle-reclaim x1, sbx-telemetry-s x1, 6 x1, sh x1, claude x1, mi-scavenger x1, Bun Pool 0 x1, Bun Pool 1 x1, Bun Pool 2 x1, HTTP Client x1, Bun Pool 3 x1, fs.watch x1, JSCWarmUp x1, JITWorker x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 0.34 1.13 1.74 1/105 28832
# toggle, skipall full:  d20_toggle: ACCEPTED, 1 result(s) d20_toggle: expected accept: ok 
# toggle, skipall nounk: d20_toggle: ACCEPTED, 1 result(s) d20_toggle: expected accept: ok 
# toggle, after full:    d20_toggle: REJECTED, code -6 (AK_ERR_TRANSCODE = -6) d20_toggle: expected reject: ok 
# toggle, after nounk:   d20_toggle: REJECTED, code -6 (AK_ERR_TRANSCODE = -6) d20_toggle: expected reject: ok 
# before-full: /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-before/ffi/poc/rust/target/release/deps/codec_suite-0c3c22650ebd039a (sha256 f42803d4ed2db116) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-before/ffi/poc/rust/target/release/deps/libak_core.so (sha256 6a924f7ed623c0a3; pvt setters exported: 0)
# before-nounk: /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-before/ffi/poc/rust/target-nounk/release/deps/codec_suite-24c79b6c40878a3a (sha256 b979e45a0d33b343) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-before/ffi/poc/rust/target-nounk/release/deps/libak_core.so (sha256 4e7b16fe57fdd19a; pvt setters exported: 0)
# after-full: /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/codec_suite-d20-after (sha256 e1956eea51697fb3) loads /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (sha256 2d1eb05921b1d438; pvt setters exported: 7)
# after-nounk: /home/user/ArmoniK.Api/ffi/poc/rust/target-nounk/release/deps/codec_suite-d20-after (sha256 938d08461a483e31) loads /home/user/ArmoniK.Api/ffi/poc/rust/target-nounk/release/deps/libak_core.so (sha256 ec8df55720bf6585; pvt setters exported: 7)
# skipall-full: /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/codec_suite-d20-skipall (sha256 9a03547cc051c165) loads /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (sha256 2d1eb05921b1d438; pvt setters exported: 7)
# skipall-nounk: /home/user/ArmoniK.Api/ffi/poc/rust/target-nounk/release/deps/codec_suite-d20-skipall (sha256 e2c8b20f9fed7ae3) loads /home/user/ArmoniK.Api/ffi/poc/rust/target-nounk/release/deps/libak_core.so (sha256 ec8df55720bf6585; pvt setters exported: 7)
# settings: AK_CAMPAIGN_GRID=core AK_CASE_ARMS=core-ffi,core-native AK_CASE_DIRS=decode-read AK_SAMPLES=10 AK_WARMUP_MS=100 AK_MEASURE_MS=250; 3 launches; CPU 1 (taskset); nothing else of this session runs meanwhile
before full launch 1: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
before nounk launch 1: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
after full launch 1: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
after nounk launch 1: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
skipall full launch 1: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
skipall nounk launch 1: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
after full launch 2: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
after nounk launch 2: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
skipall full launch 2: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
skipall nounk launch 2: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
before full launch 2: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
before nounk launch 2: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
skipall full launch 3: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
skipall nounk launch 3: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
before full launch 3: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
before nounk launch 3: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
after full launch 3: # precheck: 552 checks, 0 failures, 25 inputs, 50 cases
after nounk launch 3: # precheck: 311 checks, 0 failures, 25 inputs, 50 cases
# benchmark wall: 414 s
```

## full build, mode retain

| payload | content | core-ffi before | core-ffi after | core-ffi skipall | after/before | control: core-native after/before | skipall/after |
|---|---|---|---|---|---|---|---|
| P1.1 | ascii | 1.62 us [1.57 us - 1.64 us] | 1.60 us [1.57 us - 1.69 us] | 1.39 us [1.38 us - 1.40 us] | 1.002 [0.991 - 1.031] | 0.964 [0.963 - 1.032] | 0.873 [0.815 - 0.886] |
| P1.2 | ascii | 429.9 us [418.4 us - 434.7 us] | 411.2 us [406.7 us - 432.5 us] | 351.7 us [349.7 us - 357.7 us] | 0.983 [0.936 - 1.006] | 1.004 [0.922 - 1.031] | 0.850 [0.813 - 0.880] |
| P1.3 | ascii | 16.0 us [16.0 us - 16.4 us] | 16.2 us [15.9 us - 16.8 us] | 16.0 us [15.9 us - 16.1 us] | 1.016 [0.991 - 1.021] | 0.954 [0.873 - 1.013] | 0.996 [0.952 - 0.999] |
| P2.1 | ascii | 2.98 us [2.92 us - 3.02 us] | 2.97 us [2.95 us - 3.09 us] | 2.59 us [2.49 us - 2.61 us] | 0.997 [0.976 - 1.056] | 0.990 [0.984 - 1.004] | 0.845 [0.838 - 0.880] |
| P2.2 | ascii | 2.06 ms [1.94 ms - 2.20 ms] | 2.08 ms [2.04 ms - 2.11 ms] | 1.62 ms [1.59 ms - 1.71 ms] | 1.026 [0.942 - 1.052] | 1.027 [0.995 - 1.056] | 0.795 [0.765 - 0.809] |
| P2.2/latin1 | latin1 | 2.08 ms [1.97 ms - 2.13 ms] | 2.16 ms [2.07 ms - 2.33 ms] | 1.66 ms [1.61 ms - 1.72 ms] | 1.048 [1.016 - 1.122] | 1.012 [0.955 - 1.128] | 0.768 [0.736 - 0.781] |
| P2.2/wide | wide | 2.27 ms [2.26 ms - 2.49 ms] | 2.41 ms [2.28 ms - 2.59 ms] | 1.80 ms [1.76 ms - 1.98 ms] | 1.006 [0.967 - 1.147] | 1.065 [0.902 - 1.068] | 0.764 [0.731 - 0.789] |
| P2.3 | ascii | 1.32 ms [1.30 ms - 1.33 ms] | 1.27 ms [1.26 ms - 1.30 ms] | 1.03 ms [1.02 ms - 1.04 ms] | 0.958 [0.953 - 1.004] | 1.031 [0.987 - 1.048] | 0.807 [0.798 - 0.807] |
| P2.4 | ascii | 1.64 ms [1.62 ms - 1.79 ms] | 1.66 ms [1.65 ms - 1.74 ms] | 1.29 ms [1.29 ms - 1.33 ms] | 1.002 [0.929 - 1.077] | 1.029 [0.953 - 1.166] | 0.780 [0.738 - 0.808] |
| P2.5 | ascii | 62.6 us [60.4 us - 64.8 us] | 62.9 us [61.6 us - 64.3 us] | 52.9 us [52.7 us - 52.9 us] | 1.028 [0.950 - 1.042] | 1.009 [0.976 - 1.037] | 0.841 [0.822 - 0.855] |
| P3.1 | ascii | 32.5 us [32.3 us - 33.5 us] | 32.6 us [32.5 us - 33.1 us] | 28.6 us [28.4 us - 29.2 us] | 1.003 [0.972 - 1.024] | 1.024 [0.988 - 1.031] | 0.880 [0.870 - 0.882] |
| P4.1 | ascii | 316.8 us [314.2 us - 324.4 us] | 324.6 us [324.3 us - 329.0 us] | 237.7 us [237.3 us - 244.4 us] | 1.025 [1.000 - 1.047] | 1.009 [0.975 - 1.013] | 0.733 [0.721 - 0.753] |
| P5.1 | ascii | 119 ns [118 ns - 122 ns] | 123 ns [123 ns - 124 ns] | 97 ns [97 ns - 99 ns] | 1.028 [1.008 - 1.050] | 1.036 [1.024 - 1.045] | 0.794 [0.781 - 0.809] |
| P5.2 | ascii | 1.70 us [1.67 us - 1.71 us] | 1.68 us [1.67 us - 1.70 us] | 1.66 us [1.66 us - 1.68 us] | 0.999 [0.984 - 1.000] | 1.018 [0.989 - 1.022] | 0.985 [0.984 - 0.994] |
| P5.3 | ascii | 66.0 us [61.5 us - 66.7 us] | 65.6 us [63.1 us - 67.6 us] | 66.8 us [62.8 us - 68.5 us] | 1.014 [0.956 - 1.067] | 0.931 [0.909 - 1.108] | 0.987 [0.957 - 1.084] |
| P5.4 | ascii | 290.0 us [276.9 us - 296.3 us] | 271.2 us [258.3 us - 292.8 us] | 291.6 us [285.8 us - 305.8 us] | 0.935 [0.933 - 0.988] | 0.914 [0.905 - 0.994] | 1.127 [0.976 - 1.129] |
| P6.1 | ascii | 204.5 us [203.3 us - 209.0 us] | 203.9 us [202.1 us - 206.0 us] | 197.4 us [194.0 us - 198.6 us] | 0.988 [0.986 - 1.003] | 1.037 [0.981 - 1.089] | 0.964 [0.952 - 0.977] |
| P7.1 | ascii | 357 ns [354 ns - 362 ns] | 370 ns [366 ns - 380 ns] | 303 ns [299 ns - 308 ns] | 1.026 [1.021 - 1.075] | 0.995 [0.987 - 1.013] | 0.821 [0.787 - 0.841] |
| U-deep-u-repeated | ascii | 2.96 us [2.90 us - 2.97 us] | 2.95 us [2.89 us - 3.07 us] | 2.61 us [2.45 us - 2.63 us] | 1.015 [0.973 - 1.039] | 1.028 [0.990 - 1.098] | 0.858 [0.832 - 0.904] |
| U-nested-before | ascii | 684 ns [659 ns - 684 ns] | 688 ns [660 ns - 700 ns] | 620 ns [609 ns - 631 ns] | 1.006 [1.002 - 1.023] | 1.014 [1.004 - 1.017] | 0.917 [0.871 - 0.939] |
| U-oneof-u-repeated | ascii | 223 ns [217 ns - 224 ns] | 224 ns [224 ns - 228 ns] | 202 ns [200 ns - 204 ns] | 1.021 [0.998 - 1.031] | 1.059 [1.046 - 1.070] | 0.905 [0.879 - 0.911] |
| U-wire-DualResponse-left-as-wt5 | ascii | 313 ns [311 ns - 318 ns] | 324 ns [324 ns - 332 ns] | 278 ns [277 ns - 280 ns] | 1.042 [1.034 - 1.044] | 0.982 [0.979 - 1.018] | 0.860 [0.835 - 0.864] |
| U-wire-ListMetricsResponse-batches-as-wt0 | ascii | 1.86 us [1.84 us - 1.87 us] | 1.79 us [1.78 us - 1.82 us] | 1.76 us [1.74 us - 1.78 us] | 0.963 [0.955 - 0.992] | 1.024 [0.975 - 1.109] | 0.981 [0.954 - 0.996] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | ascii | 2.75 us [2.69 us - 2.84 us] | 2.77 us [2.70 us - 2.77 us] | 2.35 us [2.31 us - 2.40 us] | 1.008 [0.953 - 1.031] | 1.028 [1.023 - 1.032] | 0.847 [0.833 - 0.889] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | ascii | 165 ns [165 ns - 167 ns] | 177 ns [176 ns - 178 ns] | 144 ns [143 ns - 147 ns] | 1.072 [1.053 - 1.078] | 1.025 [1.018 - 1.025] | 0.820 [0.805 - 0.825] |

## no-unknown build, mode no-unknown

| payload | content | core-ffi before | core-ffi after | core-ffi skipall | after/before | control: core-native after/before | skipall/after |
|---|---|---|---|---|---|---|---|
| P1.1 | ascii | 1.06 us [1.04 us - 1.07 us] | 1.10 us [1.07 us - 1.11 us] | 898 ns [881 ns - 907 ns] | 1.038 [1.023 - 1.043] | 0.972 [0.873 - 0.979] | 0.814 [0.813 - 0.827] |
| P1.2 | ascii | 373.7 us [368.0 us - 375.0 us] | 376.0 us [374.1 us - 387.0 us] | 323.7 us [319.1 us - 332.9 us] | 1.003 [1.001 - 1.052] | 0.986 [0.978 - 0.992] | 0.853 [0.837 - 0.885] |
| P1.3 | ascii | 14.0 us [13.8 us - 14.2 us] | 14.3 us [14.2 us - 15.1 us] | 14.4 us [14.4 us - 14.5 us] | 1.036 [1.000 - 1.076] | 1.051 [1.010 - 1.089] | 1.007 [0.963 - 1.008] |
| P2.1 | ascii | 2.68 us [2.61 us - 2.73 us] | 2.75 us [2.72 us - 2.80 us] | 2.25 us [2.23 us - 2.29 us] | 1.027 [0.995 - 1.072] | 0.987 [0.987 - 0.990] | 0.817 [0.797 - 0.841] |
| P2.2 | ascii | 1.90 ms [1.83 ms - 1.97 ms] | 1.83 ms [1.82 ms - 1.84 ms] | 1.44 ms [1.43 ms - 1.51 ms] | 0.965 [0.920 - 1.005] | 0.925 [0.921 - 0.969] | 0.789 [0.786 - 0.819] |
| P2.2/latin1 | latin1 | 1.89 ms [1.85 ms - 1.93 ms] | 1.88 ms [1.84 ms - 1.92 ms] | 1.46 ms [1.44 ms - 1.48 ms] | 0.995 [0.973 - 1.013] | 0.998 [0.980 - 1.016] | 0.782 [0.758 - 0.786] |
| P2.2/wide | wide | 2.12 ms [2.06 ms - 2.47 ms] | 2.01 ms [1.96 ms - 2.05 ms] | 1.63 ms [1.53 ms - 1.74 ms] | 0.952 [0.812 - 0.970] | 0.993 [0.942 - 1.184] | 0.812 [0.745 - 0.887] |
| P2.3 | ascii | 1.17 ms [1.16 ms - 1.18 ms] | 1.17 ms [1.16 ms - 1.18 ms] | 941.3 us [940.7 us - 970.6 us] | 0.988 [0.985 - 1.015] | 0.926 [0.925 - 0.975] | 0.807 [0.797 - 0.837] |
| P2.4 | ascii | 1.56 ms [1.50 ms - 1.62 ms] | 1.50 ms [1.49 ms - 1.52 ms] | 1.24 ms [1.18 ms - 1.24 ms] | 0.963 [0.925 - 1.012] | 0.962 [0.954 - 1.068] | 0.824 [0.778 - 0.829] |
| P2.5 | ascii | 58.8 us [58.2 us - 58.9 us] | 59.3 us [59.0 us - 60.8 us] | 51.0 us [51.0 us - 52.6 us] | 1.019 [1.003 - 1.032] | 0.985 [0.977 - 1.013] | 0.861 [0.839 - 0.891] |
| P3.1 | ascii | 30.2 us [30.2 us - 30.5 us] | 30.8 us [30.3 us - 31.7 us] | 26.8 us [26.7 us - 27.3 us] | 1.011 [1.002 - 1.052] | 0.996 [0.973 - 0.999] | 0.881 [0.844 - 0.886] |
| P4.1 | ascii | 316.1 us [308.2 us - 324.6 us] | 318.6 us [306.9 us - 342.7 us] | 239.7 us [230.4 us - 251.0 us] | 1.008 [0.945 - 1.112] | 0.975 [0.966 - 1.001] | 0.781 [0.672 - 0.788] |
| P5.1 | ascii | 98 ns [98 ns - 99 ns] | 100 ns [99 ns - 100 ns] | 74 ns [73 ns - 75 ns] | 1.012 [1.009 - 1.017] | 0.986 [0.954 - 0.997] | 0.743 [0.731 - 0.749] |
| P5.2 | ascii | 1.67 us [1.67 us - 1.67 us] | 1.66 us [1.66 us - 1.67 us] | 1.65 us [1.65 us - 1.70 us] | 0.996 [0.993 - 1.000] | 0.983 [0.934 - 0.998] | 0.995 [0.995 - 1.017] |
| P5.3 | ascii | 69.2 us [67.3 us - 70.8 us] | 65.3 us [64.0 us - 71.9 us] | 66.3 us [62.3 us - 67.9 us] | 0.944 [0.904 - 1.069] | 0.959 [0.936 - 1.057] | 0.954 [0.944 - 1.036] |
| P5.4 | ascii | 279.6 us [276.2 us - 289.5 us] | 292.9 us [274.8 us - 305.3 us] | 273.2 us [270.5 us - 321.0 us] | 1.012 [0.995 - 1.092] | 0.967 [0.961 - 1.069] | 0.994 [0.924 - 1.051] |
| P6.1 | ascii | 198.1 us [195.6 us - 200.6 us] | 199.0 us [198.8 us - 199.2 us] | 189.7 us [185.6 us - 192.1 us] | 1.006 [0.992 - 1.017] | 0.957 [0.952 - 0.987] | 0.954 [0.934 - 0.964] |
| P7.1 | ascii | 328 ns [327 ns - 329 ns] | 322 ns [322 ns - 326 ns] | 258 ns [256 ns - 259 ns] | 0.984 [0.982 - 0.990] | 0.938 [0.898 - 1.058] | 0.795 [0.794 - 0.800] |
| U-deep-u-repeated | ascii | 2.59 us [2.58 us - 2.60 us] | 2.57 us [2.56 us - 2.58 us] | 2.20 us [2.14 us - 2.24 us] | 0.994 [0.984 - 0.999] | 0.973 [0.969 - 0.976] | 0.855 [0.834 - 0.868] |
| U-nested-before | ascii | 319 ns [317 ns - 340 ns] | 343 ns [341 ns - 347 ns] | 274 ns [270 ns - 275 ns] | 1.069 [1.009 - 1.094] | 0.960 [0.954 - 0.965] | 0.793 [0.792 - 0.799] |
| U-oneof-u-repeated | ascii | 134 ns [132 ns - 136 ns] | 139 ns [139 ns - 139 ns] | 119 ns [118 ns - 119 ns] | 1.034 [1.028 - 1.048] | 0.927 [0.901 - 0.935] | 0.855 [0.844 - 0.861] |
| U-wire-DualResponse-left-as-wt5 | ascii | 237 ns [236 ns - 240 ns] | 233 ns [231 ns - 233 ns] | 195 ns [193 ns - 197 ns] | 0.978 [0.970 - 0.987] | 0.934 [0.929 - 0.935] | 0.839 [0.829 - 0.852] |
| U-wire-ListMetricsResponse-batches-as-wt0 | ascii | 1.66 us [1.64 us - 1.67 us] | 1.72 us [1.68 us - 1.75 us] | 1.63 us [1.63 us - 1.65 us] | 1.048 [1.004 - 1.052] | 0.964 [0.958 - 0.970] | 0.946 [0.930 - 0.983] |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | ascii | 2.55 us [2.53 us - 2.55 us] | 2.57 us [2.51 us - 3.08 us] | 2.17 us [2.13 us - 2.18 us] | 1.009 [0.982 - 1.219] | 0.972 [0.956 - 0.973] | 0.844 [0.692 - 0.869] |
| U-wire-UploadResultDataMessage-upload-as-wt5 | ascii | 103 ns [101 ns - 107 ns] | 105 ns [105 ns - 105 ns] | 78 ns [77 ns - 79 ns] | 1.022 [0.986 - 1.042] | 0.965 [0.922 - 0.970] | 0.743 [0.730 - 0.752] |

## Summary: per-row median ratios, over the rows of each table

| build, mode | ratio | rows | median | min | max |
|---|---|---|---|---|---|
| full, retain | after/before | 25 | 1.008 | 0.935 | 1.072 |
| full, retain | native after/before | 25 | 1.018 | 0.914 | 1.065 |
| full, retain | skipall/after | 25 | 0.850 | 0.733 | 1.127 |
| nounk, no-unknown | after/before | 25 | 1.008 | 0.944 | 1.069 |
| nounk, no-unknown | native after/before | 25 | 0.972 | 0.925 | 1.051 |
| nounk, no-unknown | skipall/after | 25 | 0.839 | 0.743 | 1.007 |

