# backward-encode: encode tables (CONTAINER INSTRUMENTATION, not campaign results)

```
# backward-encode narrowed alternation, 2026-10-02T15:20:40Z. CONTAINER INSTRUMENTATION.
# cpu        Intel(R) Xeon(R) Processor @ 2.10GHz; kernel 6.18.44-fc-v51; smt notsupported (active 0); no_turbo n/a; governor cpu1 n/a; scaling min/max cpu1 n/a/n/a kHz
# isolation  cmdline: isolated='' nohz_full='n/a'
# cgroups    cpuset.cpus.effective:; this driver's cgroup /: n/a; root cpuset.cpus.isolated='n/a'; this driver's affinity 0-3
# irq        default_smp_affinity f; smp_affinity_list of /proc/irq/*: 0-3 x29, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: environment-man x10, process_api x1, vsock-console x1, idle-reclaim x1, sbx-telemetry-s x1, 6 x1, sh x1, claude x1, mi-scavenger x1, Bun Pool 0 x1, Bun Pool 1 x1, Bun Pool 2 x1, Bun Pool 3 x1, HTTP Client x1, fs.watch x1, JSCWarmUp x1, JITWorker x1, tail x1, grep x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 0.47 1.04 1.45 1/104 15839
# AK_ONLY=P6, encode, core-ffi + core-native, reused-buffer + transport-ready-tonic, hot; AK_WARMUP_MS=100 AK_MEASURE_MS=250; 3 launches per variant; CPU 1
# cH-fwdcore: /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/codec_suite-3211c86b37bffe91 (sha256 c85a94d23aa4828c) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/fwdcore/libak_core.so (sha256 577b9b2e83079ce9)
# cH-bwdcore: /home/user/ArmoniK.Api/ffi/poc/rust/target/release/deps/codec_suite-3211c86b37bffe91 (sha256 c85a94d23aa4828c) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/v1/libak_core.so (sha256 5216de8f7a664be7)
# wH-fwdcore: /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/v1/codec_suite (sha256 5871bc09622312e1) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/fwdcore/libak_core.so (sha256 577b9b2e83079ce9)
# wH-bwdcore: /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/v1/codec_suite (sha256 5871bc09622312e1) loads /tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/v1/libak_core.so (sha256 5216de8f7a664be7)
cH-fwdcore launch 1: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
cH-bwdcore launch 1: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
wH-fwdcore launch 1: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
wH-bwdcore launch 1: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
cH-fwdcore launch 2: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
cH-bwdcore launch 2: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
wH-fwdcore launch 2: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
wH-bwdcore launch 2: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
cH-fwdcore launch 3: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
cH-bwdcore launch 3: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
wH-fwdcore launch 3: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
wH-bwdcore launch 3: # precheck: 32 checks, 0 failures, 1 inputs, 8 cases
# benchmark wall: 39 s
```

Absolute ns of process CPU per encode, per variant; no ratio is formed here. Each cell: the median of the
per-launch medians (10 criterion samples per launch), [min - max] of the launch medians, then p10-p90 of
every sample of every launch. `committed` = this branch's core and binding; `backward` = the patched core
with the binding that delivers every repeated field last to first. core-native (the forward ak_rt::Enc,
unchanged) is the in-process control row of each process. End states (requirement 11): reused-buffer, and
transport-ready-tonic (core-ffi: ak_enc_take_owned's moved buffer; core-native: Enc::take). Input: hot.

## P6.1

| arm | mode | end state | cH-fwdcore | cH-bwdcore | wH-fwdcore | wH-bwdcore |
|---|---|---|---|---|---|---|
| core-ffi | drop | reused-buffer | 62.9 us [62.6 us - 63.9 us] (p10-p90 61.3 us - 64.8 us, 3 launches) | 75.1 us [74.3 us - 75.9 us] (p10-p90 74.0 us - 80.4 us, 3 launches) | 63.9 us [63.8 us - 64.3 us] (p10-p90 63.4 us - 65.9 us, 3 launches) | 75.2 us [75.1 us - 75.7 us] (p10-p90 74.9 us - 76.4 us, 3 launches) |
| core-ffi | drop | transport-ready-tonic | 61.7 us [61.5 us - 62.0 us] (p10-p90 61.2 us - 62.9 us, 3 launches) | 74.6 us [74.4 us - 76.5 us] (p10-p90 73.7 us - 77.6 us, 3 launches) | 64.3 us [63.8 us - 65.0 us] (p10-p90 63.6 us - 65.3 us, 3 launches) | 76.4 us [76.4 us - 76.9 us] (p10-p90 76.2 us - 79.5 us, 3 launches) |
| core-ffi | retain | reused-buffer | 61.1 us [60.4 us - 63.8 us] (p10-p90 60.1 us - 64.6 us, 3 launches) | 77.0 us [74.2 us - 77.1 us] (p10-p90 74.1 us - 78.8 us, 3 launches) | 64.4 us [64.3 us - 66.0 us] (p10-p90 63.9 us - 67.2 us, 3 launches) | 77.3 us [76.7 us - 77.6 us] (p10-p90 76.4 us - 80.3 us, 3 launches) |
| core-ffi | retain | transport-ready-tonic | 61.7 us [61.1 us - 62.8 us] (p10-p90 61.1 us - 63.2 us, 3 launches) | 75.7 us [75.3 us - 77.0 us] (p10-p90 74.8 us - 79.4 us, 3 launches) | 64.9 us [64.7 us - 66.4 us] (p10-p90 64.4 us - 69.0 us, 3 launches) | 76.9 us [76.9 us - 77.2 us] (p10-p90 76.5 us - 78.0 us, 3 launches) |
| core-native | drop | reused-buffer | 66.9 us [65.3 us - 67.1 us] (p10-p90 65.2 us - 69.3 us, 3 launches) | 65.3 us [64.8 us - 66.7 us] (p10-p90 64.7 us - 68.4 us, 3 launches) | 57.5 us [56.3 us - 58.9 us] (p10-p90 56.3 us - 60.0 us, 3 launches) | 57.2 us [56.4 us - 60.3 us] (p10-p90 56.1 us - 60.6 us, 3 launches) |
| core-native | drop | transport-ready-tonic | 65.7 us [65.4 us - 67.3 us] (p10-p90 65.1 us - 68.3 us, 3 launches) | 65.4 us [65.3 us - 66.7 us] (p10-p90 65.0 us - 70.4 us, 3 launches) | 58.1 us [56.6 us - 59.2 us] (p10-p90 56.5 us - 59.8 us, 3 launches) | 57.1 us [56.5 us - 57.8 us] (p10-p90 56.5 us - 59.1 us, 3 launches) |
| core-native | retain | reused-buffer | 71.5 us [69.3 us - 71.7 us] (p10-p90 68.6 us - 74.6 us, 3 launches) | 70.3 us [70.0 us - 71.0 us] (p10-p90 68.8 us - 72.9 us, 3 launches) | 63.9 us [61.2 us - 63.9 us] (p10-p90 61.1 us - 64.7 us, 3 launches) | 61.2 us [61.0 us - 66.6 us] (p10-p90 60.7 us - 68.9 us, 3 launches) |
| core-native | retain | transport-ready-tonic | 71.3 us [68.6 us - 72.3 us] (p10-p90 68.2 us - 75.9 us, 3 launches) | 71.7 us [71.1 us - 72.2 us] (p10-p90 69.3 us - 78.8 us, 3 launches) | 61.7 us [61.5 us - 61.8 us] (p10-p90 61.2 us - 62.8 us, 3 launches) | 61.6 us [61.2 us - 61.9 us] (p10-p90 61.0 us - 63.5 us, 3 launches) |

