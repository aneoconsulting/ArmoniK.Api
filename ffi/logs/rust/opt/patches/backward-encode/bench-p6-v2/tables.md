# backward-encode: encode tables (CONTAINER INSTRUMENTATION, not campaign results)

```
# backward-encode narrowed alternation, 2026-10-02T15:19:37Z. CONTAINER INSTRUMENTATION.
# cpu        Intel(R) Xeon(R) Processor @ 2.10GHz; kernel 6.18.44-fc-v51; smt notsupported (active 0); no_turbo n/a; governor cpu1 n/a; scaling min/max cpu1 n/a/n/a kHz
# isolation  cmdline: isolated='' nohz_full='n/a'
# cgroups    cpuset.cpus.effective:; this driver's cgroup /: n/a; root cpuset.cpus.isolated='n/a'; this driver's affinity 0-3
# irq        default_smp_affinity f; smp_affinity_list of /proc/irq/*: 0-3 x29, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: environment-man x10, HeapHelper x3, process_api x1, vsock-console x1, idle-reclaim x1, sbx-telemetry-s x1, 6 x1, sh x1, claude x1, mi-scavenger x1, Bun Pool 0 x1, Bun Pool 1 x1, Bun Pool 2 x1, Bun Pool 3 x1, HTTP Client x1, fs.watch x1, JSCWarmUp x1, JITWorker x1, tail x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 0.73 1.19 1.52 1/106 15586
# AK_ONLY=P6, encode, core-ffi + core-native, reused-buffer + transport-ready-tonic, hot; AK_WARMUP_MS=100 AK_MEASURE_MS=250; 3 launches per variant; CPU 1
# committed: /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/codec_suite-3211c86b37bffe91 (sha256 c85a94d23aa4828c) loads /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (sha256 577b9b2e83079ce9)
# v1: /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/v1/codec_suite (sha256 5871bc09622312e1) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/v1/libak_core.so (sha256 5216de8f7a664be7)
# v2: /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-bwd/ffi/poc/rust/target/release/deps/codec_suite-7e804e611118a090 (sha256 5871bc09622312e1) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/wt-bwd/ffi/poc/rust/target/release/deps/libak_core.so (sha256 a1d62b1e2e4d4619)
committed launch 1: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
v1 launch 1: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
v2 launch 1: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
committed launch 2: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
v1 launch 2: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
v2 launch 2: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
committed launch 3: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
v1 launch 3: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
v2 launch 3: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
# benchmark wall: 29 s
```

Absolute ns of process CPU per encode, per variant; no ratio is formed here. Each cell: the median of the
per-launch medians (10 criterion samples per launch), [min - max] of the launch medians, then p10-p90 of
every sample of every launch. `committed` = this branch's core and binding; `backward` = the patched core
with the binding that delivers every repeated field last to first. core-native (the forward ak_rt::Enc,
unchanged) is the in-process control row of each process. End states (requirement 11): reused-buffer, and
transport-ready-tonic (core-ffi: ak_enc_take_owned's moved buffer; core-native: Enc::take). Input: hot.

## P6.1

| arm | mode | end state | committed | v1 | v2 |
|---|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 60.9 us [60.3 us - 61.1 us] (p10-p90 60.3 us - 61.3 us, 3 launches) | 76.2 us [75.3 us - 79.6 us] (p10-p90 75.2 us - 82.1 us, 3 launches) | 81.3 us [80.8 us - 84.9 us] (p10-p90 80.3 us - 89.7 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 61.6 us [61.3 us - 63.3 us] (p10-p90 61.1 us - 64.3 us, 3 launches) | 76.4 us [76.0 us - 77.8 us] (p10-p90 75.7 us - 78.4 us, 3 launches) | 83.3 us [81.1 us - 85.8 us] (p10-p90 80.9 us - 89.3 us, 3 launches) |
| core-ffi | retain | reused-buffer | 60.7 us [60.5 us - 61.9 us] (p10-p90 60.2 us - 62.3 us, 3 launches) | 76.6 us [76.0 us - 77.0 us] (p10-p90 75.9 us - 78.6 us, 3 launches) | 80.8 us [80.4 us - 82.4 us] (p10-p90 80.2 us - 83.0 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 61.8 us [61.4 us - 62.6 us] (p10-p90 61.1 us - 63.8 us, 3 launches) | 76.7 us [76.3 us - 77.6 us] (p10-p90 76.1 us - 77.8 us, 3 launches) | 82.2 us [81.3 us - 86.0 us] (p10-p90 80.3 us - 86.9 us, 3 launches) |
| core-native | drop | reused-buffer | 65.4 us [65.1 us - 66.2 us] (p10-p90 64.9 us - 69.8 us, 3 launches) | 56.5 us [56.3 us - 57.5 us] (p10-p90 55.9 us - 58.0 us, 3 launches) | 57.7 us [56.6 us - 58.6 us] (p10-p90 56.5 us - 59.8 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 65.7 us [65.0 us - 66.1 us] (p10-p90 64.9 us - 66.5 us, 3 launches) | 56.4 us [56.2 us - 57.1 us] (p10-p90 56.1 us - 57.7 us, 3 launches) | 57.9 us [56.3 us - 58.3 us] (p10-p90 56.1 us - 60.3 us, 3 launches) |
| core-native | retain | reused-buffer | 68.7 us [68.2 us - 70.1 us] (p10-p90 67.8 us - 70.2 us, 3 launches) | 60.8 us [60.8 us - 61.0 us] (p10-p90 60.5 us - 61.6 us, 3 launches) | 61.7 us [61.6 us - 62.1 us] (p10-p90 60.9 us - 63.1 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 69.2 us [68.8 us - 70.4 us] (p10-p90 68.4 us - 70.7 us, 3 launches) | 61.0 us [60.7 us - 61.4 us] (p10-p90 60.5 us - 61.6 us, 3 launches) | 62.5 us [62.4 us - 63.3 us] (p10-p90 61.3 us - 64.8 us, 3 launches) |

