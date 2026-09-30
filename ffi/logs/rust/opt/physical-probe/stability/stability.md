# in-process comparison: commit a093104a; 2026-09-30T19:20:03Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: sddm-helper x1, systemd x1, .kwin_wayland-w x1, QDBusConnection x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 1.84 2.02 1.68 1/904 456505
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 456238, pinned socket
# cells A,A2,Cf,Cf-cb,Cf-zc,Cf-zcw (direction c: A,A2,Cf,Cf-cb) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 c54k1 c54k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 28
# condition head: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 66a405bee46ded27, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env AK_PROBE_SKIP_MISSING=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stack: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 abd544bfe4a99be1, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_PROBE_SKIP_MISSING=1,AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 858 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: sddm-helper x1, systemd x1, .kwin_wayland-w x1, QDBusConnection x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.55 2.66 2.29 2/904 476623

## 1. Absolute client CPU and wall per call, pooled over every process: median [p10-p90]


### d16k1

| cell | head CPU ms | head wall ms | head processes | stack CPU ms | stack wall ms | stack processes |
|---|---|---|---|---|---|---|
| A | 7.81 [7.35-8.71] | 8.71 [8.19-9.02] | 28 | 7.90 [7.42-8.78] | 8.60 [8.14-8.98] | 28 |
| A2 | 7.88 [7.38-8.73] | 8.70 [8.19-9.01] | 28 | 7.79 [7.34-8.63] | 8.78 [8.18-9.11] | 28 |
| Cf | 8.20 [7.92-8.98] | 8.09 [7.27-8.52] | 28 | 8.34 [8.00-9.05] | 8.07 [7.36-8.51] | 28 |
| Cf-cb | 9.19 [8.59-10.19] | 8.07 [7.53-8.57] | 28 | 9.18 [8.63-10.04] | 7.97 [7.54-8.59] | 28 |
| Cf-zc | -- | -- | -- | 6.28 [6.03-6.77] | 7.52 [6.95-8.18] | 28 |
| Cf-zcw | -- | -- | -- | 6.40 [6.11-6.85] | 7.47 [6.96-8.22] | 28 |

### d16k8

| cell | head CPU ms | head wall ms | head processes | stack CPU ms | stack wall ms | stack processes |
|---|---|---|---|---|---|---|
| A | 9.33 [8.85-10.19] | 8.70 [8.43-9.23] | 28 | 9.35 [8.78-10.16] | 8.73 [8.41-9.24] | 28 |
| A2 | 9.32 [8.86-10.26] | 8.72 [8.33-9.14] | 28 | 9.32 [8.82-10.24] | 8.72 [8.40-9.22] | 28 |
| Cf | 9.54 [9.31-9.95] | 8.30 [8.07-8.50] | 28 | 9.66 [9.46-9.96] | 8.29 [8.07-8.52] | 28 |
| Cf-cb | 9.97 [9.53-10.70] | 8.31 [8.06-8.57] | 28 | 9.66 [9.36-10.04] | 8.29 [8.03-8.50] | 28 |
| Cf-zc | -- | -- | -- | 6.44 [6.22-6.79] | 7.98 [7.73-8.18] | 28 |
| Cf-zcw | -- | -- | -- | 6.45 [6.21-6.75] | 7.98 [7.77-8.23] | 28 |

### d4k1

| cell | head CPU ms | head wall ms | head processes | stack CPU ms | stack wall ms | stack processes |
|---|---|---|---|---|---|---|
| A | 1.98 [1.86-2.52] | 2.37 [2.26-2.51] | 28 | 2.01 [1.86-2.55] | 2.43 [2.29-2.50] | 28 |
| A2 | 1.96 [1.85-2.52] | 2.38 [2.28-2.49] | 28 | 1.96 [1.85-2.52] | 2.39 [2.28-2.48] | 28 |
| Cf | 2.05 [1.96-2.14] | 2.37 [2.20-2.50] | 28 | 2.02 [1.94-2.11] | 2.39 [2.24-2.49] | 28 |
| Cf-cb | 2.17 [2.07-2.48] | 2.41 [2.26-2.51] | 28 | 2.17 [2.08-2.49] | 2.42 [2.26-2.51] | 28 |
| Cf-zc | -- | -- | -- | 1.45 [1.38-1.56] | 2.17 [2.07-2.25] | 28 |
| Cf-zcw | -- | -- | -- | 1.47 [1.39-1.60] | 2.15 [2.05-2.25] | 28 |

### c54k1

| cell | head CPU ms | head wall ms | head processes | stack CPU ms | stack wall ms | stack processes |
|---|---|---|---|---|---|---|
| A | 1.95 [1.86-2.05] | 2.93 [2.69-3.85] | 28 | 1.96 [1.86-2.05] | 3.00 [2.72-4.13] | 28 |
| A2 | 1.96 [1.86-2.05] | 2.95 [2.71-4.08] | 28 | 1.96 [1.86-2.07] | 3.03 [2.70-4.50] | 28 |
| Cf | 2.00 [1.90-2.09] | 3.12 [2.81-4.25] | 28 | 2.01 [1.91-2.13] | 3.16 [2.82-4.52] | 28 |
| Cf-cb | 1.99 [1.89-2.10] | 3.07 [2.79-4.11] | 28 | 2.01 [1.91-2.12] | 3.20 [2.84-4.33] | 28 |

### c54k8

| cell | head CPU ms | head wall ms | head processes | stack CPU ms | stack wall ms | stack processes |
|---|---|---|---|---|---|---|
| A | 2.34 [2.03-3.09] | 2.52 [2.33-2.66] | 28 | 2.40 [2.04-3.15] | 2.51 [2.35-2.67] | 28 |
| A2 | 2.36 [2.04-3.14] | 2.52 [2.37-2.65] | 28 | 2.33 [2.04-3.19] | 2.50 [2.35-2.69] | 28 |
| Cf | 2.55 [2.43-2.70] | 2.60 [2.46-2.73] | 28 | 2.53 [2.42-2.69] | 2.59 [2.45-2.72] | 28 |
| Cf-cb | 2.18 [2.10-2.46] | 2.51 [2.39-2.66] | 28 | 2.22 [2.12-2.49] | 2.54 [2.37-2.72] | 28 |

## 2. Per-process gap to A (cell median - A median of the same process), ms: median, p10-p90, min..max over the processes


### d16k1

| cell | head: median | head: p10-p90 | head: min..max | stack: median | stack: p10-p90 | stack: min..max |
|---|---|---|---|---|---|---|
| A2 | +0.06 | -0.45..+0.35 | -0.75..+0.69 | -0.11 | -0.61..+0.32 | -0.72..+0.49 |
| Cf | +0.37 | -0.04..+1.06 | -0.27..+1.68 | +0.47 | +0.03..+1.00 | -0.08..+1.29 |
| Cf-cb | +1.22 | +0.81..+1.89 | +0.64..+2.37 | +1.19 | +0.74..+1.56 | +0.66..+2.12 |
| Cf-zc | -- | -- | -- | -1.65 | -2.15..-1.22 | -2.39..-1.05 |
| Cf-zcw | -- | -- | -- | -1.57 | -2.08..-1.07 | -2.14..-0.83 |

### d16k8

| cell | head: median | head: p10-p90 | head: min..max | stack: median | stack: p10-p90 | stack: min..max |
|---|---|---|---|---|---|---|
| A2 | -0.03 | -0.46..+0.31 | -0.51..+0.37 | +0.03 | -0.37..+0.30 | -0.58..+0.38 |
| Cf | +0.19 | -0.11..+0.48 | -0.17..+0.54 | +0.32 | +0.13..+0.62 | -0.21..+0.68 |
| Cf-cb | +0.62 | +0.34..+1.02 | +0.29..+1.13 | +0.36 | +0.08..+0.57 | -0.29..+0.67 |
| Cf-zc | -- | -- | -- | -2.88 | -3.18..-2.60 | -3.31..-2.47 |
| Cf-zcw | -- | -- | -- | -2.93 | -3.20..-2.64 | -3.35..-2.41 |

### d4k1

| cell | head: median | head: p10-p90 | head: min..max | stack: median | stack: p10-p90 | stack: min..max |
|---|---|---|---|---|---|---|
| A2 | -0.03 | -0.31..+0.34 | -0.36..+0.41 | +0.00 | -0.33..+0.22 | -0.40..+0.28 |
| Cf | -0.03 | -0.23..+0.16 | -0.25..+0.17 | -0.14 | -0.21..+0.12 | -0.28..+0.17 |
| Cf-cb | +0.07 | -0.12..+0.29 | -0.14..+0.31 | +0.02 | -0.10..+0.29 | -0.13..+0.31 |
| Cf-zc | -- | -- | -- | -0.71 | -0.80..-0.37 | -0.90..-0.32 |
| Cf-zcw | -- | -- | -- | -0.67 | -0.79..-0.40 | -0.84..-0.38 |

### c54k1

| cell | head: median | head: p10-p90 | head: min..max | stack: median | stack: p10-p90 | stack: min..max |
|---|---|---|---|---|---|---|
| A2 | -0.00 | -0.04..+0.05 | -0.05..+0.08 | +0.01 | -0.04..+0.06 | -0.09..+0.07 |
| Cf | +0.04 | -0.00..+0.08 | -0.01..+0.09 | +0.06 | +0.01..+0.10 | -0.03..+0.19 |
| Cf-cb | +0.04 | +0.01..+0.09 | -0.01..+0.11 | +0.06 | -0.01..+0.10 | -0.02..+0.15 |

### c54k8

| cell | head: median | head: p10-p90 | head: min..max | stack: median | stack: p10-p90 | stack: min..max |
|---|---|---|---|---|---|---|
| A2 | +0.04 | -0.19..+0.16 | -0.23..+0.29 | -0.07 | -0.32..+0.18 | -0.35..+0.22 |
| Cf | +0.20 | +0.03..+0.32 | -0.01..+0.44 | +0.15 | -0.04..+0.32 | -0.08..+0.48 |
| Cf-cb | -0.16 | -0.31..+0.04 | -0.36..+0.27 | -0.18 | -0.32..+0.11 | -0.42..+0.16 |

## 3. Drift: per-process medians in run order (ms), and the medians of the first and last third of the processes

- head d16k1 A: first third 7.79, last third 7.67; run order: 8.03 7.72 8.46 8.01 7.35 7.75 7.79 7.89 7.71 8.50 7.69 7.44 8.07 7.85 7.61 8.17 8.38 7.64 8.09 7.58 7.67 8.10 7.89 7.60 8.37 7.58 7.84 7.51
- head d16k1 A2: first third 7.92, last third 7.93; run order: 7.89 8.06 7.71 8.20 7.64 7.92 8.05 7.92 7.55 8.06 7.83 7.73 8.10 8.04 8.30 8.02 7.86 7.60 7.63 7.93 8.34 8.03 7.68 7.77 7.97 7.90 7.93 7.43
- head d16k1 Cf: first third 8.41, last third 8.17; run order: 8.41 7.96 8.56 8.67 9.03 8.80 8.21 8.18 8.30 8.24 8.18 8.06 8.19 8.12 7.96 8.14 8.34 8.26 8.18 8.06 8.20 8.17 8.07 8.17 8.13 8.27 8.20 8.58
- head d16k1 Cf-cb: first third 9.22, last third 9.20; run order: 9.17 9.47 9.51 9.25 9.41 9.22 8.98 9.00 9.19 9.25 9.22 8.70 9.00 8.71 9.51 8.98 9.56 8.85 9.28 9.13 9.40 8.74 9.22 8.91 9.20 9.35 8.89 9.88
- head d16k8 A: first third 9.37, last third 9.30; run order: 9.25 9.37 9.49 9.61 9.09 9.57 9.32 9.75 9.22 9.44 9.18 9.28 9.31 9.35 9.31 9.52 9.51 9.08 9.43 8.97 9.47 9.10 9.60 9.30 9.28 9.34 9.57 9.16
- head d16k8 A2: first third 9.35, last third 9.28; run order: 9.34 9.69 9.42 9.12 9.35 9.11 9.52 9.24 9.59 9.26 9.34 9.09 9.28 9.58 9.26 9.66 9.39 9.39 9.36 9.08 9.33 9.35 9.21 9.28 9.54 9.03 9.36 9.20
- head d16k8 Cf: first third 9.56, last third 9.51; run order: 9.60 9.56 9.72 9.55 9.52 9.42 9.63 9.58 9.49 9.57 9.58 9.43 9.53 9.53 9.48 9.51 9.40 9.58 9.59 9.51 9.46 9.59 9.58 9.47 9.51 9.68 9.62 9.38
- head d16k8 Cf-cb: first third 10.04, last third 10.05; run order: 10.04 10.14 10.10 9.90 9.74 9.92 9.90 10.11 10.24 9.86 9.85 9.62 10.01 9.89 9.78 9.88 10.23 10.11 9.94 10.09 10.05 10.05 10.08 9.99 9.90 10.10 9.91 10.14
- head d4k1 A: first third 1.99, last third 2.03; run order: 1.89 2.25 1.94 2.27 2.13 1.94 2.22 1.98 1.99 2.28 1.98 2.18 1.92 2.24 2.16 2.12 1.89 2.18 2.24 1.92 1.89 2.12 2.23 1.89 2.09 1.92 2.12 2.03
- head d4k1 A2: first third 2.18, last third 2.00; run order: 2.24 1.92 2.17 2.29 1.89 2.21 2.18 1.92 2.33 1.92 2.08 2.25 2.21 1.99 1.87 1.89 1.90 1.87 2.20 2.22 1.86 2.00 1.93 2.30 2.13 2.01 1.89 1.93
- head d4k1 Cf: first third 2.06, last third 2.05; run order: 2.05 2.35 2.06 2.08 2.02 2.05 2.06 2.07 2.10 2.05 2.02 1.98 2.05 1.99 2.04 2.06 2.05 2.06 2.00 2.05 2.02 2.04 2.07 2.06 2.05 2.00 2.05 2.01
- head d4k1 Cf-cb: first third 2.18, last third 2.17; run order: 2.18 2.27 2.16 2.22 2.17 2.17 2.12 2.19 2.22 2.16 2.10 2.21 2.17 2.10 2.21 2.19 2.16 2.20 2.11 2.22 2.17 2.20 2.17 2.18 2.13 2.17 2.16 2.15
- head c54k1 A: first third 1.97, last third 1.94; run order: 1.97 2.02 1.99 1.97 1.94 1.99 1.88 1.94 2.02 1.95 1.94 1.91 1.96 1.90 1.97 1.95 1.95 1.92 1.94 1.94 1.99 1.94 1.92 1.96 1.95 1.94 1.94 1.93
- head c54k1 A2: first third 1.98, last third 1.94; run order: 1.94 2.01 1.98 1.98 1.92 1.98 1.96 1.98 1.98 1.95 1.93 1.94 1.95 1.92 1.99 1.98 2.02 1.98 1.92 1.99 1.94 1.89 1.92 1.96 1.93 1.90 1.96 1.96
- head c54k1 Cf: first third 2.03, last third 2.00; run order: 1.97 2.03 2.03 1.99 1.99 2.03 1.94 2.03 2.06 2.00 1.99 1.99 2.00 1.95 1.98 1.99 1.95 1.92 1.99 2.03 2.00 1.99 1.97 1.97 2.02 1.99 2.02 2.02
- head c54k1 Cf-cb: first third 2.01, last third 1.99; run order: 1.99 2.05 2.01 2.08 1.97 2.06 1.92 2.01 2.07 2.02 1.96 1.98 2.01 1.99 1.98 1.97 1.98 1.91 1.97 2.04 2.00 1.99 1.97 1.97 1.96 1.97 2.00 2.00
- head c54k8 A: first third 2.32, last third 2.33; run order: 2.33 2.51 2.33 2.30 2.53 2.31 2.30 2.30 2.32 2.40 2.23 2.16 2.34 2.32 2.46 2.28 2.46 2.35 2.46 2.43 2.33 2.40 2.31 2.53 2.33 2.38 2.08 2.28
- head c54k8 A2: first third 2.42, last third 2.35; run order: 2.42 2.65 2.51 2.59 2.41 2.42 2.26 2.39 2.25 2.35 2.39 2.27 2.32 2.25 2.45 2.33 2.34 2.49 2.47 2.42 2.42 2.20 2.39 2.35 2.35 2.15 2.17 2.33
- head c54k8 Cf: first third 2.56, last third 2.54; run order: 2.55 2.50 2.56 2.58 2.56 2.53 2.60 2.59 2.57 2.50 2.55 2.57 2.60 2.51 2.49 2.56 2.52 2.48 2.60 2.61 2.51 2.54 2.53 2.56 2.50 2.55 2.52 2.55
- head c54k8 Cf-cb: first third 2.28, last third 2.18; run order: 2.36 2.19 2.16 2.32 2.26 2.34 2.28 2.37 2.15 2.12 2.15 2.20 2.33 2.15 2.17 2.13 2.16 2.14 2.17 2.12 2.21 2.16 2.18 2.17 2.13 2.26 2.35 2.19
- stack d16k1 A: first third 7.99, last third 7.97; run order: 7.66 7.99 8.50 8.01 8.15 7.55 8.08 7.49 7.84 7.55 8.53 7.95 8.07 8.32 7.65 7.96 7.60 7.84 8.03 8.15 8.03 8.22 7.88 7.66 8.23 7.86 7.70 7.97
- stack d16k1 A2: first third 7.86, last third 7.59; run order: 7.77 7.61 8.00 8.33 8.24 7.76 7.86 7.66 8.33 7.65 7.92 7.61 8.01 7.98 7.51 7.69 7.96 8.04 7.64 7.49 7.55 7.50 7.59 7.53 8.14 8.17 7.64 7.96
- stack d16k1 Cf: first third 8.41, last third 8.24; run order: 8.21 8.17 8.42 8.56 8.41 8.55 8.54 8.18 8.21 8.17 8.55 8.54 8.68 8.40 8.81 8.19 8.60 8.54 8.24 8.18 8.24 8.35 9.17 8.33 8.83 8.23 8.18 8.24
- stack d16k1 Cf-cb: first third 9.19, last third 9.03; run order: 9.17 8.87 9.23 9.56 9.31 9.66 9.19 8.80 8.95 9.00 9.42 9.15 9.42 9.27 9.12 9.52 9.34 9.39 9.38 8.81 9.18 8.95 8.87 8.94 9.21 9.03 9.04 9.03
- stack d16k1 Cf-zc: first third 6.26, last third 6.20; run order: 6.31 6.14 6.11 6.19 6.68 6.50 6.37 6.22 6.26 6.33 6.62 6.68 6.46 6.14 6.16 6.14 6.39 6.32 6.30 6.30 6.17 6.07 6.63 6.39 6.37 6.16 6.20 6.14
- stack d16k1 Cf-zcw: first third 6.41, last third 6.32; run order: 6.83 6.29 6.36 6.43 6.41 6.40 6.52 6.42 6.26 6.21 6.45 6.49 6.55 6.26 6.58 6.35 6.58 6.45 6.46 6.32 6.18 6.14 6.59 6.41 6.54 6.28 6.33 6.27
- stack d16k8 A: first third 9.34, last third 9.47; run order: 9.10 9.29 9.53 9.34 9.49 9.27 9.03 9.38 9.35 8.99 9.21 9.22 9.47 9.15 9.77 9.43 9.32 9.10 9.23 9.48 9.31 9.66 9.38 9.49 9.54 9.08 9.47 9.14
- stack d16k8 A2: first third 9.31, last third 9.27; run order: 9.30 9.29 9.25 9.45 9.53 9.31 9.19 9.40 9.55 9.29 9.33 9.59 9.54 9.35 9.26 9.46 9.29 9.14 9.15 9.45 9.27 9.29 9.47 9.24 8.96 9.21 9.14 9.45
- stack d16k8 Cf: first third 9.66, last third 9.62; run order: 9.72 9.66 9.70 9.87 9.62 9.70 9.58 9.64 9.64 9.62 9.53 9.76 9.67 9.83 9.56 9.65 9.65 9.59 9.66 9.62 9.58 9.56 9.75 9.74 9.69 9.65 9.61 9.58
- stack d16k8 Cf-cb: first third 9.69, last third 9.65; run order: 9.63 9.64 9.78 9.69 9.69 9.73 9.67 9.68 9.71 9.54 9.61 9.88 9.76 9.66 9.73 9.78 9.69 9.50 9.61 9.69 9.80 9.37 9.66 9.62 9.71 9.65 9.55 9.65
- stack d16k8 Cf-zc: first third 6.51, last third 6.44; run order: 6.41 6.51 6.54 6.75 6.32 6.47 6.34 6.65 6.63 6.52 6.32 6.34 6.42 6.36 6.46 6.54 6.35 6.55 6.29 6.48 6.44 6.40 6.32 6.41 6.36 6.44 6.44 6.50
- stack d16k8 Cf-zcw: first third 6.41, last third 6.43; run order: 6.41 6.32 6.62 6.61 6.51 6.29 6.28 6.41 6.78 6.58 6.51 6.27 6.27 6.35 6.61 6.56 6.50 6.31 6.23 6.29 6.52 6.50 6.43 6.27 6.19 6.28 6.43 6.50
- stack d4k1 A: first third 2.00, last third 2.12; run order: 2.00 1.94 2.27 1.97 1.93 2.19 2.22 1.93 2.27 2.30 2.24 2.16 2.20 2.16 1.90 1.95 1.89 2.17 2.23 2.23 1.89 2.03 1.90 1.95 2.12 2.22 2.19 2.15
- stack d4k1 A2: first third 1.97, last third 2.01; run order: 1.94 1.87 1.94 2.25 2.15 2.22 2.21 1.97 1.97 1.90 2.17 1.88 2.27 2.23 1.98 1.90 1.92 2.20 1.94 1.89 2.11 2.23 2.11 1.96 1.91 2.01 1.97 2.19
- stack d4k1 Cf: first third 2.03, last third 2.02; run order: 2.02 2.03 2.06 2.09 2.00 2.01 2.02 2.10 2.07 2.02 2.01 1.99 2.05 1.99 1.96 2.00 1.97 2.03 2.03 2.02 1.96 2.03 2.01 2.04 2.02 2.03 2.04 2.00
- stack d4k1 Cf-cb: first third 2.19, last third 2.17; run order: 2.14 2.16 2.19 2.20 2.23 2.16 2.19 2.21 2.17 2.16 2.20 2.17 2.20 2.12 2.19 2.26 2.18 2.17 2.16 2.15 2.14 2.17 2.17 2.17 2.18 2.13 2.18 2.18
- stack d4k1 Cf-zc: first third 1.46, last third 1.43; run order: 1.44 1.49 1.51 1.46 1.56 1.46 1.40 1.46 1.47 1.40 1.48 1.46 1.44 1.41 1.46 1.42 1.54 1.44 1.43 1.46 1.39 1.38 1.57 1.41 1.42 1.47 1.47 1.43
- stack d4k1 Cf-zcw: first third 1.51, last third 1.44; run order: 1.45 1.45 1.53 1.49 1.53 1.51 1.43 1.51 1.54 1.45 1.48 1.48 1.46 1.47 1.52 1.43 1.49 1.50 1.43 1.44 1.44 1.39 1.50 1.40 1.49 1.49 1.43 1.42
- stack c54k1 A: first third 1.98, last third 1.95; run order: 1.96 2.03 1.99 1.98 1.96 1.93 1.98 2.00 2.00 1.94 1.96 1.94 1.98 1.95 1.93 1.95 1.86 1.90 1.96 1.91 1.97 1.95 1.93 1.96 1.94 1.98 1.97 1.91
- stack c54k1 A2: first third 1.97, last third 1.95; run order: 1.93 2.05 2.01 2.04 1.95 1.93 1.94 2.01 1.97 1.99 1.96 1.97 1.89 1.91 1.94 1.95 1.93 1.94 1.95 1.94 1.99 2.00 1.98 1.94 1.95 1.93 1.95 1.91
- stack c54k1 Cf: first third 2.01, last third 2.01; run order: 1.95 1.99 2.07 2.06 1.98 2.00 2.01 2.06 2.03 2.01 1.99 2.04 1.99 1.99 2.03 2.04 1.98 1.96 2.02 2.01 2.00 2.00 2.12 2.04 1.96 2.02 2.01 1.99
- stack c54k1 Cf-cb: first third 2.03, last third 2.00; run order: 1.99 2.02 2.07 2.03 2.02 2.03 1.97 2.03 2.05 2.01 1.99 2.04 2.00 2.01 2.02 2.04 2.01 1.98 1.98 1.99 2.03 2.00 1.99 2.04 2.00 1.96 2.03 2.01
- stack c54k8 A: first third 2.42, last third 2.41; run order: 2.24 2.49 2.41 2.46 2.42 2.11 2.44 2.27 2.48 2.32 2.11 2.52 2.40 2.50 2.41 2.42 2.24 2.48 2.36 2.44 2.58 2.29 2.41 2.55 2.25 2.22 2.47 2.25
- stack c54k8 A2: first third 2.37, last third 2.23; run order: 2.42 2.37 2.37 2.52 2.40 2.31 2.35 2.11 2.40 2.28 2.32 2.17 2.45 2.26 2.14 2.27 2.37 2.48 2.31 2.28 2.23 2.18 2.08 2.34 2.43 2.17 2.43 2.16
- stack c54k8 Cf: first third 2.52, last third 2.50; run order: 2.57 2.48 2.57 2.49 2.52 2.50 2.63 2.52 2.58 2.46 2.59 2.47 2.48 2.46 2.61 2.49 2.54 2.50 2.58 2.56 2.50 2.46 2.60 2.60 2.49 2.48 2.58 2.50
- stack c54k8 Cf-cb: first third 2.18, last third 2.25; run order: 2.14 2.18 2.20 2.35 2.15 2.28 2.15 2.14 2.24 2.36 2.22 2.31 2.27 2.32 2.26 2.17 2.35 2.20 2.16 2.27 2.16 2.25 2.19 2.23 2.30 2.29 2.15 2.27

## 4. Voluntary context switches / minor faults per call (medians over rounds)


### d16k1

| cell | head | stack |
|---|---|---|
| A | 95 / 0.0 | 112 / 0.0 |
| A2 | 104 / 0.0 | 97 / 0.0 |
| Cf | 89 / 0.1 | 96 / 0.1 |
| Cf-cb | 226 / 0.2 | 232 / 0.2 |
| Cf-zc | -- | 112 / 0.1 |
| Cf-zcw | -- | 122 / 0.1 |

### d16k8

| cell | head | stack |
|---|---|---|
| A | 79 / 176.3 | 75 / 192.0 |
| A2 | 80 / 192.0 | 80 / 160.8 |
| Cf | 72 / 1.2 | 74 / 0.5 |
| Cf-cb | 110 / 192.6 | 108 / 0.8 |
| Cf-zc | -- | 81 / 0.6 |
| Cf-zcw | -- | 80 / 0.5 |

### d4k1

| cell | head | stack |
|---|---|---|
| A | 32 / 0.0 | 37 / 0.0 |
| A2 | 26 / 0.0 | 29 / 0.0 |
| Cf | 30 / 0.0 | 26 / 0.0 |
| Cf-cb | 48 / 0.1 | 46 / 0.1 |
| Cf-zc | -- | 16 / 0.0 |
| Cf-zcw | -- | 20 / 0.0 |

### c54k1

| cell | head | stack |
|---|---|---|
| A | 14 / 0.0 | 14 / 0.0 |
| A2 | 14 / 0.0 | 14 / 0.0 |
| Cf | 18 / 0.1 | 18 / 0.1 |
| Cf-cb | 16 / 0.0 | 16 / 0.0 |

### c54k8

| cell | head | stack |
|---|---|---|
| A | 27 / 96.6 | 29 / 128.3 |
| A2 | 31 / 96.4 | 27 / 96.3 |
| Cf | 22 / 0.5 | 22 / 0.6 |
| Cf-cb | 30 / 0.1 | 31 / 0.2 |

## Note: server affinity during this campaign

This campaign made no per-process check of the server's or the client's affinity (the checks
were added to gen/inproc.sh afterwards). The owner's re-pinning of user processes to
0,9,10,19 moved the C++ campaign's server between its rounds 10 and 11:
`ffi/logs/cpp/opt/physical-probe/stability/run1/runner.log` lines 26 and 28
(`machine_after_round 10 2026-09-30T19:15:34Z`, `machine_after_round 11 2026-09-30T19:16:41Z`).
This campaign started at 19:20:03Z (21:20:03 CEST), after that C++ run ended (19:20:01Z), with a
server that gen/inproc.sh started and pinned to 5-8,15-18 at its own start; so the re-pinning
predates this campaign's server. That is an inference from the two logs' times, not a check.
