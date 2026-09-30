# in-process comparison: commit f449d7fd + UNCOMMITTED; 2026-09-30T22:22:20Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: sddm-helper x1, systemd x1, .kwin_wayland-w x1, QDBusConnection x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 0.68 0.49 0.62 1/894 589242
# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts
# server also on TCP 127.0.0.1:33597 (pinned configuration, TCP_NODELAY on accept)
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 588930, pinned socket
# cells A,Df,Df-1f,Cf,Cf-cb,Cf-zc (direction c: A,Df,Df-1f,Cf,Cf-cb) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 c54k1 c54k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2; d16k16 5 x 1 batch of 16 warm 1; d16k32 4 x 1 x 32 warm 1; c54k16 6 x 2 x 16 warm 1; c54k32 5 x 2 x 32 warm 1); repetitions 3
# condition uds: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 e175b93e6c0ae95b, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_HOST_WORKERS=8,AK_CORE_WORKERS=8,AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition tcp: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 e175b93e6c0ae95b, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_RPC_TARGET=http://@TCP@,AK_EXPECT_NODELAY=1,AK_HOST_WORKERS=8,AK_CORE_WORKERS=8,AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 147 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: sddm-helper x1, systemd x1, .kwin_wayland-w x1, QDBusConnection x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.27 1.27 0.91 1/890 593159

## d16k1

| cell | uds: CPU ms | uds: wall ms | uds: gap to A per rep (range) | uds: vcs / ics / minflt | tcp: CPU ms | tcp: wall ms | tcp: gap to A per rep (range) | tcp: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 7.69 [7.37-8.42] | 8.84 [8.68-9.46] |  | 82.4 / 0.0 / 0.0 | 9.93 [9.83-10.06] | 14.93 [14.81-15.01] |  | 35.9 / 0.4 / 0.0 |
| Df | 8.01 [7.76-8.62] | 9.20 [9.01-10.07] | +0.24 +0.33 +0.40 (0.16) | 172.7 / 0.0 / 0.2 | 10.03 [9.94-10.17] | 14.75 [14.49-14.94] | +0.11 +0.11 +0.08 (0.03) | 49.3 / 0.4 / 0.1 |
| Cf | 8.41 [8.05-9.02] | 8.13 [7.46-8.79] | +0.79 +0.73 +0.73 (0.05) | 90.8 / 0.0 / 0.1 | 10.49 [10.43-10.61] | 13.46 [13.40-13.56] | +0.57 +0.56 +0.57 (0.01) | 45.4 / 0.2 / 0.1 |
| Cf-cb | 9.11 [8.65-10.13] | 7.96 [7.51-8.88] | +1.57 +1.07 +1.48 (0.50) | 236.0 / 0.0 / 0.1 | 10.94 [10.81-11.26] | 13.62 [13.53-13.78] | +0.96 +1.03 +1.04 (0.08) | 78.1 / 0.4 / 0.3 |
| Cf-zc | 6.39 [6.11-6.86] | 7.83 [7.13-8.33] | -1.19 -1.61 -1.31 (0.42) | 121.3 / 0.0 / 0.2 | 8.59 [8.52-8.69] | 13.28 [13.23-13.36] | -1.32 -1.36 -1.33 (0.04) | 47.4 / 0.2 / 0.1 |
| Df-1f | 7.52 [7.28-8.11] | 8.92 [8.23-9.76] | +0.22 -0.61 -0.09 (0.83) | 60.0 / 0.0 / 0.2 | 9.88 [9.79-10.01] | 14.87 [14.81-14.92] | -0.04 -0.07 -0.04 (0.04) | 36.7 / 0.4 / 0.1 |

## d16k8

| cell | uds: CPU ms | uds: wall ms | uds: gap to A per rep (range) | uds: vcs / ics / minflt | tcp: CPU ms | tcp: wall ms | tcp: gap to A per rep (range) | tcp: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 9.14 [8.79-9.86] | 8.72 [8.37-9.16] |  | 75.1 / 0.1 / 160.4 | 11.07 [10.52-12.50] | 13.07 [12.85-13.38] |  | 34.3 / 0.2 / 175.9 |
| Df | 9.43 [9.12-9.85] | 8.62 [8.34-8.84] | +0.47 +0.10 +0.37 (0.37) | 112.4 / 0.0 / 1.3 | 10.59 [10.36-10.95] | 12.94 [12.67-13.12] | -0.31 -0.56 -0.30 (0.26) | 39.2 / 0.1 / 16.7 |
| Cf | 9.74 [9.46-9.96] | 8.36 [8.13-8.55] | +0.57 +0.57 +0.76 (0.19) | 73.1 / 0.1 / 0.6 | 11.40 [8.69-11.66] | 13.13 [9.56-13.39] | +0.61 +0.44 -2.20 (2.81) | 41.0 / 0.2 / 0.2 |
| Cf-cb | 9.63 [9.40-9.88] | 8.42 [8.06-8.73] | +0.36 +0.42 +0.69 (0.33) | 103.2 / 0.1 / 0.6 | 11.22 [10.91-11.47] | 13.03 [12.91-13.32] | +0.26 +0.00 +0.38 (0.38) | 57.1 / 0.2 / 0.8 |
| Cf-zc | 6.39 [6.20-6.67] | 8.04 [7.84-8.27] | -2.87 -3.01 -2.44 (0.57) | 76.0 / 0.0 / 0.7 | 8.38 [7.29-8.65] | 12.25 [11.11-12.49] | -2.26 -2.75 -3.60 (1.34) | 44.6 / 0.1 / 0.4 |
| Df-1f | 8.98 [8.64-9.35] | 8.52 [8.28-8.84] | -0.21 -0.27 +0.02 (0.29) | 78.3 / 0.0 / 0.4 | 10.77 [7.94-10.98] | 13.03 [9.17-13.22] | -2.73 -0.32 -0.11 (2.62) | 35.2 / 0.1 / 0.5 |

## d4k1

| cell | uds: CPU ms | uds: wall ms | uds: gap to A per rep (range) | uds: vcs / ics / minflt | tcp: CPU ms | tcp: wall ms | tcp: gap to A per rep (range) | tcp: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 1.96 [1.86-2.54] | 2.46 [2.37-2.52] |  | 33.2 / 0.0 / 0.0 | 2.56 [2.52-2.66] | 3.75 [3.70-3.87] |  | 13.0 / 0.1 / 0.0 |
| Df | 1.93 [1.86-2.36] | 2.41 [2.34-2.48] | -0.04 -0.02 -0.12 (0.10) | 17.4 / 0.0 / 0.1 | 2.61 [2.57-2.69] | 3.74 [3.70-3.80] | +0.06 +0.05 +0.04 (0.03) | 14.8 / 0.1 / 0.0 |
| Cf | 2.04 [1.97-2.12] | 2.45 [2.20-2.49] | -0.17 +0.17 -0.01 (0.34) | 26.1 / 0.0 / 0.1 | 2.65 [2.63-2.71] | 3.76 [3.74-3.80] | +0.11 +0.10 +0.07 (0.04) | 17.4 / 0.1 / 0.0 |
| Cf-cb | 2.20 [2.10-2.50] | 2.42 [2.25-2.49] | -0.02 +0.39 +0.14 (0.41) | 50.0 / 0.0 / 0.0 | 2.75 [2.70-2.82] | 3.75 [3.73-3.80] | +0.19 +0.19 +0.17 (0.02) | 28.2 / 0.1 / 0.1 |
| Cf-zc | 1.40 [1.35-1.49] | 2.17 [2.06-2.24] | -0.81 -0.53 -0.59 (0.28) | 14.7 / 0.0 / 0.0 | 2.12 [2.08-2.19] | 3.52 [3.49-3.55] | -0.43 -0.43 -0.46 (0.03) | 13.6 / 0.1 / 0.0 |
| Df-1f | 1.95 [1.84-2.49] | 2.34 [2.23-2.46] | -0.03 -0.05 +0.16 (0.21) | 38.2 / 0.0 / 0.0 | 2.57 [2.53-2.65] | 3.74 [3.69-3.80] | +0.03 +0.01 +0.00 (0.02) | 13.0 / 0.1 / 0.0 |

## c54k1

| cell | uds: CPU ms | uds: wall ms | uds: gap to A per rep (range) | uds: vcs / ics / minflt | tcp: CPU ms | tcp: wall ms | tcp: gap to A per rep (range) | tcp: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 1.96 [1.87-2.06] | 3.19 [2.87-4.03] |  | 15.1 / 0.0 / 0.0 | 2.56 [2.52-2.61] | 4.24 [4.18-4.27] |  | 10.5 / 0.1 / 0.0 |
| Df | 1.92 [1.82-2.07] | 4.18 [2.83-4.98] | -0.09 +0.02 -0.00 (0.11) | 13.5 / 0.0 / 0.0 | 2.57 [2.51-2.62] | 4.25 [4.21-4.31] | -0.02 +0.04 +0.01 (0.07) | 8.9 / 0.1 / 0.1 |
| Cf | 1.98 [1.88-2.07] | 3.21 [2.91-3.79] | +0.03 +0.03 +0.03 (0.00) | 18.3 / 0.0 / 0.1 | 2.62 [2.58-2.66] | 4.32 [4.26-4.35] | +0.05 +0.06 +0.06 (0.01) | 15.1 / 0.1 / 0.1 |
| Cf-cb | 2.01 [1.92-2.11] | 3.30 [3.03-3.79] | +0.04 +0.07 +0.07 (0.03) | 17.4 / 0.0 / 0.0 | 2.63 [2.59-2.67] | 4.35 [4.32-4.39] | +0.06 +0.06 +0.08 (0.02) | 11.8 / 0.1 / 0.1 |
| Df-1f | 1.90 [1.81-2.03] | 2.99 [2.67-4.63] | -0.08 -0.13 +0.02 (0.15) | 13.5 / 0.0 / 0.0 | 2.56 [2.51-2.62] | 4.25 [4.20-4.29] | -0.02 +0.03 -0.01 (0.05) | 9.7 / 0.1 / 0.0 |

## c54k8

| cell | uds: CPU ms | uds: wall ms | uds: gap to A per rep (range) | uds: vcs / ics / minflt | tcp: CPU ms | tcp: wall ms | tcp: gap to A per rep (range) | tcp: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.34 [2.06-2.98] | 2.46 [2.21-2.66] |  | 24.4 / 0.0 / 80.3 | 2.65 [2.49-3.35] | 3.58 [3.45-3.72] |  | 9.7 / 0.1 / 96.3 |
| Df | 2.12 [2.06-2.40] | 2.51 [2.37-2.61] | -0.23 -0.17 -0.12 (0.11) | 27.4 / 0.0 / 0.1 | 2.63 [2.56-2.72] | 3.56 [3.42-3.67] | -0.05 +0.03 -0.00 (0.09) | 10.6 / 0.0 / 0.1 |
| Cf | 2.57 [2.46-2.71] | 2.59 [2.48-2.71] | +0.24 +0.21 +0.24 (0.03) | 22.0 / 0.0 / 0.5 | 3.09 [3.04-3.24] | 3.60 [3.52-3.71] | +0.45 +0.56 +0.42 (0.14) | 13.7 / 0.1 / 0.5 |
| Cf-cb | 2.21 [2.14-2.48] | 2.48 [2.36-2.65] | -0.15 -0.08 -0.12 (0.07) | 32.6 / 0.0 / 0.1 | 2.62 [2.09-2.74] | 3.52 [2.84-3.71] | -0.54 +0.13 -0.04 (0.66) | 15.0 / 0.0 / 0.1 |
| Df-1f | 2.15 [2.06-2.39] | 2.51 [2.27-2.63] | -0.22 -0.07 -0.24 (0.17) | 32.0 / 0.0 / 0.1 | 2.61 [2.50-2.73] | 3.59 [3.49-3.74] | -0.05 -0.01 -0.03 (0.04) | 10.6 / 0.0 / 0.1 |
