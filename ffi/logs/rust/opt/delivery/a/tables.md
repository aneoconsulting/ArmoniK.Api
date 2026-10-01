# in-process comparison: commit 99c18cd1 + UNCOMMITTED; 2026-10-01T21:34:00Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 1.97 2.07 2.25 1/944 1016551
# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts
# transport tcp (client target http://127.0.0.1:40077, TCP_NODELAY read back on every client socket); base env AK_PROBE_TASKCLOCK=1 AK_PROBE_SERVER_PID=1016150 AK_RPC_TARGET=http://127.0.0.1:40077 AK_EXPECT_NODELAY=1
# server also on TCP 127.0.0.1:40077 (pinned configuration, TCP_NODELAY on accept)
# netfilter (read-only): modules nft_log nft_limit xt_limit xt_NFLOG nfnetlink_log xt_physdev xt_multiport nf_conntrack_netlink xt_mark xt_nfacct nfnetlink_acct xt_comment xt_set ip_set xt_addrtype xt_CHECKSUM xt_MASQUERADE xt_conntrack ipt_REJECT nf_reject_ipv4 xt_tcpudp nft_compat nft_chain_nat nf_tables nfnetlink ip6_udp_tunnel nfit xt_nat br_netfilter nf_nat bridge nf_conntrack nf_defrag_ipv6 nf_defrag_ipv4 
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 1016150, pinned configuration, transport tcp
# cells A,A-blk,A-cb,A-q (direction c: A,A-blk,A-cb,A-q) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 c54k1 c54k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2; d16k16 5 x 1 batch of 16 warm 1; d16k32 4 x 1 x 32 warm 1; c54k16 6 x 2 x 16 warm 1; c54k32 5 x 2 x 32 warm 1); repetitions 3
# condition a-stock: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-deliv/release/stream_probe (sha256 38f9fc7b27c4e040), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition a-hosttoo: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-deliv-h2batch/release/stream_probe (sha256 f18f20bb0fc7f43f), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 112 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.53 2.24 2.29 2/950 1019746

## d16k1

| cell | a-stock: CPU ms | a-stock: wall ms | a-stock: gap to A per rep (range) | a-stock: vcs / ics / minflt | a-hosttoo: CPU ms | a-hosttoo: wall ms | a-hosttoo: gap to A per rep (range) | a-hosttoo: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 9.98 [9.85-10.22] | 14.83 [14.67-15.04] |  | 37.0 / 0.4 / 0.0 | 6.50 [6.15-6.67] | 9.59 [8.61-10.39] |  | 15.4 / 0.0 / 0.0 |
| A-blk | 10.08 [9.87-10.96] | 14.70 [14.49-15.02] | +0.12 +0.05 +0.08 (0.07) | 39.9 / 0.2 / 0.0 | 6.75 [6.56-6.90] | 8.46 [8.29-9.17] | +0.18 +0.61 +0.11 (0.50) | 19.4 / 0.0 / 0.0 |
| A-cb | 9.94 [9.61-10.18] | 14.79 [14.04-14.97] | +0.06 -0.24 -0.05 (0.30) | 37.1 / 0.1 / 0.0 | 6.73 [6.20-7.30] | 9.17 [8.11-9.69] | +0.18 +0.67 -0.31 (0.97) | 21.0 / 0.0 / 0.1 |
| A-q | 9.99 [9.50-10.29] | 14.75 [13.35-14.89] | +0.03 -0.36 +0.08 (0.44) | 36.9 / 0.2 / 0.1 | 6.61 [6.16-6.79] | 9.04 [8.23-9.97] | +0.10 +0.47 -0.36 (0.83) | 17.6 / 0.0 / 0.0 |

## d16k8

| cell | a-stock: CPU ms | a-stock: wall ms | a-stock: gap to A per rep (range) | a-stock: vcs / ics / minflt | a-hosttoo: CPU ms | a-hosttoo: wall ms | a-hosttoo: gap to A per rep (range) | a-hosttoo: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 11.21 [10.86-12.60] | 13.20 [12.96-14.00] |  | 34.9 / 0.2 / 128.2 | 7.35 [6.87-7.90] | 8.02 [7.74-9.74] |  | 18.6 / 0.1 / 96.0 |
| A-blk | 10.93 [8.32-12.40] | 13.13 [9.33-13.61] | -0.09 -0.20 -2.76 (2.66) | 38.1 / 0.2 / 112.3 | 7.30 [6.84-7.67] | 8.51 [7.89-10.53] | +0.05 -0.43 +0.30 (0.73) | 24.3 / 0.1 / 64.2 |
| A-cb | 11.02 [8.23-12.21] | 13.16 [9.12-14.07] | -2.84 +0.04 -0.01 (2.88) | 34.5 / 0.1 / 160.3 | 6.90 [6.60-7.60] | 10.01 [8.27-11.05] | -0.51 -0.61 +0.20 (0.81) | 31.2 / 0.0 / 96.2 |
| A-q | 11.09 [10.46-12.22] | 13.33 [12.90-13.93] | -0.26 -0.16 -0.01 (0.25) | 34.0 / 0.2 / 176.4 | 6.94 [6.64-7.52] | 10.22 [9.88-10.67] | -0.26 -0.56 -0.22 (0.34) | 33.5 / 0.0 / 144.4 |

## d4k1

| cell | a-stock: CPU ms | a-stock: wall ms | a-stock: gap to A per rep (range) | a-stock: vcs / ics / minflt | a-hosttoo: CPU ms | a-hosttoo: wall ms | a-hosttoo: gap to A per rep (range) | a-hosttoo: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.57 [2.54-2.63] | 3.68 [3.64-3.78] |  | 13.1 / 0.1 / 0.0 | 1.68 [1.61-1.82] | 2.66 [2.61-4.80] |  | 7.6 / 0.0 / 0.0 |
| A-blk | 2.58 [2.52-2.70] | 3.76 [3.69-3.93] | -0.03 -0.01 +0.04 (0.07) | 15.6 / 0.1 / 0.0 | 1.70 [1.63-1.80] | 2.89 [2.77-4.15] | +0.03 -0.00 +0.03 (0.03) | 9.5 / 0.0 / 0.0 |
| A-cb | 2.59 [2.54-2.66] | 3.75 [3.67-3.86] | +0.02 +0.00 +0.03 (0.03) | 14.2 / 0.1 / 0.0 | 1.71 [1.64-1.79] | 2.86 [2.55-4.63] | -0.00 +0.04 +0.08 (0.08) | 7.4 / 0.0 / 0.0 |
| A-q | 2.60 [2.55-2.68] | 3.80 [3.74-3.87] | +0.04 +0.00 +0.02 (0.04) | 14.0 / 0.1 / 0.0 | 1.72 [1.65-1.78] | 2.87 [2.66-4.78] | +0.06 +0.02 +0.05 (0.03) | 7.2 / 0.0 / 0.0 |

## c54k1

| cell | a-stock: CPU ms | a-stock: wall ms | a-stock: gap to A per rep (range) | a-stock: vcs / ics / minflt | a-hosttoo: CPU ms | a-hosttoo: wall ms | a-hosttoo: gap to A per rep (range) | a-hosttoo: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.57 [2.54-2.62] | 4.19 [4.11-4.28] |  | 11.1 / 0.1 / 0.0 | 1.66 [1.62-1.72] | 2.98 [2.91-3.04] |  | 10.1 / 0.0 / 0.0 |
| A-blk | 2.52 [2.47-2.62] | 4.15 [4.11-4.37] | -0.08 -0.05 -0.04 (0.04) | 14.2 / 0.1 / 0.0 | 1.68 [1.61-1.73] | 3.63 [3.25-4.36] | +0.04 +0.01 -0.02 (0.06) | 10.0 / 0.0 / 0.1 |
| A-cb | 2.59 [2.53-2.63] | 4.22 [4.13-4.27] | +0.01 -0.02 +0.03 (0.05) | 12.7 / 0.1 / 0.0 | 1.66 [1.61-1.72] | 3.14 [2.99-3.70] | +0.00 +0.03 -0.03 (0.06) | 6.7 / 0.0 / 0.0 |
| A-q | 2.59 [2.54-2.64] | 4.25 [4.21-4.29] | -0.00 +0.01 +0.04 (0.04) | 12.7 / 0.1 / 0.0 | 1.70 [1.64-1.75] | 3.10 [3.01-4.09] | +0.05 +0.01 +0.02 (0.05) | 6.6 / 0.0 / 0.0 |

## c54k8

| cell | a-stock: CPU ms | a-stock: wall ms | a-stock: gap to A per rep (range) | a-stock: vcs / ics / minflt | a-hosttoo: CPU ms | a-hosttoo: wall ms | a-hosttoo: gap to A per rep (range) | a-hosttoo: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.60 [2.48-3.34] | 3.59 [3.49-3.70] |  | 9.3 / 0.1 / 96.4 | 1.73 [1.54-2.41] | 2.79 [2.48-3.06] |  | 6.4 / 0.0 / 96.4 |
| A-blk | 2.65 [2.12-2.82] | 3.54 [2.94-3.69] | +0.08 -0.48 +0.12 (0.60) | 14.0 / 0.1 / 0.2 | 1.67 [1.61-1.88] | 2.79 [2.42-3.01] | +0.07 -0.13 -0.07 (0.19) | 10.7 / 0.1 / 0.2 |
| A-cb | 2.65 [2.57-3.57] | 3.55 [3.48-3.70] | +0.04 +0.05 +0.10 (0.06) | 9.0 / 0.0 / 64.2 | 1.80 [1.56-2.66] | 2.68 [2.50-2.96] | +0.14 +0.06 +0.09 (0.08) | 5.7 / 0.0 / 144.5 |
| A-q | 2.59 [2.00-3.17] | 3.45 [2.88-3.63] | -0.49 +0.11 -0.13 (0.59) | 10.0 / 0.0 / 80.2 | 1.84 [1.55-2.41] | 2.71 [2.47-3.05] | +0.31 +0.08 -0.11 (0.42) | 6.2 / 0.0 / 128.3 |
