# in-process comparison: commit 6d47eb91; 2026-09-30T19:45:56Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: sddm-helper x1, systemd x1, .kwin_wayland-w x1, QDBusConnection x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: kworker/6:0-mm_percpu_wq x1, .kwin_wayland-w x1, ; loadavg 2.10 1.97 2.01 3/906 485571
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 485326, pinned socket
# cells A,Cf,Cf-cb,Cn-1rt (direction c: A,Cf,Cf-cb,Cn-1rt) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 c54k1 c54k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition off: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 abd544bfe4a99be1, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition inline: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 abd544bfe4a99be1, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_CB_INLINE=1,AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 80 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: sddm-helper x1, systemd x1, .kwin_wayland-w x1, QDBusConnection x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: .kwin_wayland-w x1, ; loadavg 2.82 2.21 2.09 2/910 487286

## d16k1

| cell | off: CPU ms | off: wall ms | off: gap to A per rep (range) | off: vcs / ics / minflt | inline: CPU ms | inline: wall ms | inline: gap to A per rep (range) | inline: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 7.80 [7.34-8.59] | 8.92 [8.37-9.83] |  | 97.0 / 0.0 / 0.0 | 7.68 [7.32-8.23] | 8.91 [8.34-10.15] |  | 80.7 / 0.0 / 0.0 |
| Cf | 8.26 [8.04-8.73] | 8.65 [7.85-9.34] | +0.74 +0.31 +0.37 (0.43) | 85.9 / 0.0 / 0.1 | 8.33 [8.05-9.04] | 8.26 [7.52-8.59] | +0.56 +1.07 +0.59 (0.51) | 87.2 / 0.0 / 0.1 |
| Cf-cb | 9.34 [8.60-10.21] | 8.48 [8.19-8.92] | +1.38 +0.82 +1.94 (1.13) | 214.7 / 0.0 / 0.2 | 9.06 [8.63-10.00] | 8.38 [7.65-9.52] | +1.18 +1.25 +1.82 (0.65) | 200.9 / 0.0 / 0.2 |
| Cn-1rt | 8.07 [7.65-9.02] | 8.64 [8.26-9.35] | +0.80 +0.40 -0.07 (0.87) | 105.1 / 0.0 / 0.1 | 8.18 [7.75-9.01] | 8.66 [8.30-9.60] | +0.33 +0.45 +0.84 (0.50) | 98.6 / 0.0 / 0.1 |

## d16k8

| cell | off: CPU ms | off: wall ms | off: gap to A per rep (range) | off: vcs / ics / minflt | inline: CPU ms | inline: wall ms | inline: gap to A per rep (range) | inline: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 9.29 [8.90-10.15] | 8.87 [8.48-9.70] |  | 78.6 / 0.1 / 208.1 | 9.26 [8.80-10.09] | 8.70 [8.31-9.17] |  | 87.9 / 0.0 / 176.0 |
| Cf | 9.65 [9.48-9.98] | 8.28 [8.07-8.60] | +0.38 +0.54 +0.05 (0.49) | 75.9 / 0.1 / 0.5 | 9.67 [9.46-9.92] | 8.30 [8.03-8.54] | +0.34 +0.27 +0.58 (0.31) | 77.9 / 0.1 / 0.5 |
| Cf-cb | 9.67 [9.31-9.85] | 8.32 [8.12-8.95] | +0.45 +0.33 +0.14 (0.30) | 107.8 / 0.1 / 0.9 | 9.82 [9.47-10.09] | 8.25 [8.05-8.59] | +0.68 +0.23 +0.73 (0.50) | 107.1 / 0.1 / 0.7 |
| Cn-1rt | 9.79 [9.45-10.67] | 8.39 [8.17-8.59] | +0.40 +0.69 +0.43 (0.29) | 90.6 / 0.0 / 112.2 | 9.71 [9.22-10.84] | 8.43 [8.15-8.64] | +0.56 +0.33 +0.55 (0.23) | 92.3 / 0.0 / 64.1 |

## d4k1

| cell | off: CPU ms | off: wall ms | off: gap to A per rep (range) | off: vcs / ics / minflt | inline: CPU ms | inline: wall ms | inline: gap to A per rep (range) | inline: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 1.97 [1.84-2.53] | 2.44 [2.40-2.64] |  | 32.2 / 0.0 / 0.0 | 2.15 [1.87-2.56] | 2.43 [2.34-2.57] |  | 46.2 / 0.0 / 0.0 |
| Cf | 2.04 [1.96-2.15] | 2.40 [2.23-2.46] | +0.17 -0.06 -0.16 (0.33) | 29.0 / 0.0 / 0.1 | 2.04 [1.96-2.12] | 2.39 [2.24-2.54] | +0.07 -0.17 -0.13 (0.24) | 28.5 / 0.0 / 0.1 |
| Cf-cb | 2.17 [2.10-2.45] | 2.45 [2.26-2.56] | +0.30 +0.07 -0.01 (0.32) | 41.1 / 0.0 / 0.1 | 2.10 [2.01-2.18] | 2.30 [2.23-2.51] | +0.11 -0.12 -0.05 (0.23) | 32.2 / 0.0 / 0.1 |
| Cn-1rt | 2.01 [1.92-2.09] | 2.29 [2.19-2.48] | +0.13 -0.11 -0.15 (0.28) | 27.0 / 0.0 / 0.1 | 2.01 [1.93-2.08] | 2.40 [2.37-2.48] | +0.02 -0.19 -0.17 (0.21) | 29.0 / 0.0 / 0.1 |

## c54k1

| cell | off: CPU ms | off: wall ms | off: gap to A per rep (range) | off: vcs / ics / minflt | inline: CPU ms | inline: wall ms | inline: gap to A per rep (range) | inline: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 1.94 [1.84-2.03] | 3.02 [2.68-3.65] |  | 12.5 / 0.0 / 0.0 | 1.94 [1.85-2.02] | 2.92 [2.69-4.10] |  | 14.1 / 0.0 / 0.0 |
| Cf | 2.01 [1.91-2.10] | 3.48 [2.87-3.85] | +0.09 +0.09 +0.04 (0.06) | 17.5 / 0.0 / 0.1 | 1.99 [1.89-2.08] | 3.10 [2.80-3.78] | +0.05 +0.06 +0.02 (0.04) | 17.2 / 0.0 / 0.0 |
| Cf-cb | 1.97 [1.87-2.11] | 2.98 [2.79-4.41] | +0.04 +0.10 -0.03 (0.13) | 15.6 / 0.0 / 0.1 | 1.99 [1.90-2.06] | 3.09 [2.79-3.71] | +0.07 +0.02 +0.06 (0.05) | 16.0 / 0.0 / 0.1 |
| Cn-1rt | 1.92 [1.82-2.04] | 3.36 [2.68-4.32] | +0.06 +0.00 -0.09 (0.15) | 13.5 / 0.0 / 0.0 | 1.90 [1.79-2.01] | 2.96 [2.67-4.03] | -0.04 -0.02 -0.10 (0.08) | 11.8 / 0.0 / 0.0 |

## c54k8

| cell | off: CPU ms | off: wall ms | off: gap to A per rep (range) | off: vcs / ics / minflt | inline: CPU ms | inline: wall ms | inline: gap to A per rep (range) | inline: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.33 [2.04-3.06] | 2.52 [2.40-2.76] |  | 22.2 / 0.0 / 112.3 | 2.43 [2.04-3.12] | 2.47 [2.32-2.62] |  | 33.6 / 0.0 / 80.4 |
| Cf | 2.53 [2.44-2.67] | 2.58 [2.41-2.69] | +0.33 +0.23 +0.10 (0.22) | 22.6 / 0.0 / 0.4 | 2.55 [2.43-2.69] | 2.56 [2.40-2.71] | +0.08 +0.12 +0.28 (0.20) | 20.8 / 0.0 / 0.4 |
| Cf-cb | 2.23 [2.12-2.50] | 2.51 [2.41-2.68] | +0.04 +0.08 -0.32 (0.40) | 30.3 / 0.0 / 0.2 | 2.25 [2.14-2.49] | 2.52 [2.39-2.73] | -0.22 -0.10 -0.04 (0.18) | 35.6 / 0.0 / 0.2 |
| Cn-1rt | 2.08 [2.01-2.30] | 2.48 [2.17-2.68] | -0.14 -0.19 -0.40 (0.26) | 20.9 / 0.0 / 0.1 | 2.19 [2.04-2.45] | 2.57 [2.37-2.66] | -0.19 -0.30 -0.14 (0.15) | 29.9 / 0.0 / 0.1 |
