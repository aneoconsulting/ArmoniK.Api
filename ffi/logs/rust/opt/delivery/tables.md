# Response-delivery comparison (gen/delivery.sh): raw figures

## session a

```
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
```

## Notes

- Machine: after the owner's re-pin following the second suspend; checked before the run: 47 IRQs on 0,9-10,19 and 2 on 0-19 (the headers' irq line); sleep inhibitor `sleep infinity` (block, sleep) in place.
- Cell definitions: stream_probe.rs, `delivery_batch` (A-blk, A-cb, A-q, Cf-q). A = tonic tasks spawned on the cell's runtime and awaited by block_on (the probe's reference A); Cf = the core's blocking delivery from k caller threads; Cf-cb = the core's callback into a tokio oneshot awaited by tasks on the cell's runtime (Rust's reference callback cell); Cf-q = the core's completion queue, one queue per cell, one caller thread issues k calls and drains, one encode context per in-flight slot.
- Checks before timing (checks/checks.log): every delivery cell on every probe/core pair, d/16 and d/4 at k 1 and 8 on the server's check path (data byte count and SHA-256 of the messages as received), c/P5.4 at k 1 and 8 (response length); the plant control (one byte more expected) fails every new cell with the planted error. The plant first PASSED on A-blk: its call did not go through the planted expectation; fixed (A-blk now builds cell A's call through the same function as A-cb and A-q) before timing.


### a: d16k1

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | a-hosttoo | 7.72 [6.88-7.85] | 6.53 [6.25-6.62] | 9.59 [8.61-10.39] | 15.4 / 0.0 | 76 | 9.01 | 24 |
| A | a-stock | 15.18 [15.00-15.78] | 10.02 [9.91-10.47] | 14.83 [14.67-15.04] | 37.0 / 0.4 | 1035 | 11.08 | 24 |
| A-blk | a-hosttoo | 7.99 [7.92-8.14] | 6.75 [6.66-6.86] | 8.46 [8.29-9.17] | 19.4 / 0.0 | 77 | 8.69 | 24 |
| A-blk | a-stock | 15.31 [14.87-15.63] | 10.31 [9.97-10.54] | 14.70 [14.49-15.02] | 39.9 / 0.2 | 1040 | 11.02 | 24 |
| A-cb | a-hosttoo | 8.02 [6.87-8.42] | 6.76 [6.32-7.11] | 9.17 [8.11-9.69] | 21.0 / 0.0 | 78 | 8.73 | 24 |
| A-cb | a-stock | 15.08 [14.07-15.29] | 10.01 [9.67-10.16] | 14.79 [14.04-14.97] | 37.1 / 0.1 | 1036 | 11.00 | 24 |
| A-q | a-hosttoo | 7.90 [6.69-8.07] | 6.65 [6.19-6.78] | 9.04 [8.23-9.97] | 17.6 / 0.0 | 77 | 8.69 | 24 |
| A-q | a-stock | 15.00 [13.13-15.17] | 10.05 [9.34-10.29] | 14.75 [13.35-14.89] | 36.9 / 0.2 | 1037 | 11.11 | 24 |

### a: d16k8

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | a-hosttoo | 8.49 [7.62-8.79] | 7.33 [7.02-7.69] | 8.02 [7.77-9.60] | 18.6 / 0.1 | 43 | 13.94 | 18 |
| A | a-stock | 15.49 [15.12-16.50] | 11.15 [10.99-12.22] | 13.20 [12.99-13.49] | 34.9 / 0.2 | 1032 | 21.53 | 18 |
| A-blk | a-hosttoo | 8.20 [7.42-8.53] | 7.30 [6.92-7.50] | 8.51 [8.06-10.45] | 24.3 / 0.1 | 45 | 13.94 | 18 |
| A-blk | a-stock | 15.15 [9.39-17.09] | 10.97 [8.43-12.67] | 13.13 [9.39-13.46] | 38.1 / 0.2 | 1034 | 21.54 | 18 |
| A-cb | a-hosttoo | 7.65 [7.29-8.22] | 6.92 [6.71-7.31] | 10.01 [8.28-11.04] | 31.2 / 0.0 | 64 | 13.53 | 18 |
| A-cb | a-stock | 15.51 [9.09-16.36] | 11.23 [8.24-11.74] | 13.16 [9.29-13.92] | 34.5 / 0.1 | 1033 | 19.98 | 18 |
| A-q | a-hosttoo | 7.47 [7.17-7.90] | 7.00 [6.68-7.42] | 10.22 [9.89-10.59] | 33.5 / 0.0 | 64 | 13.77 | 18 |
| A-q | a-stock | 15.47 [14.45-16.01] | 11.29 [10.58-11.73] | 13.33 [12.91-13.88] | 34.0 / 0.2 | 1033 | 21.08 | 18 |

### a: d4k1

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | a-hosttoo | 1.95 [1.86-2.02] | 1.70 [1.67-1.74] | 2.66 [2.61-4.80] | 7.6 / 0.0 | 21 | 2.26 | 24 |
| A | a-stock | 3.79 [3.78-3.89] | 2.59 [2.57-2.67] | 3.68 [3.64-3.78] | 13.1 / 0.1 | 261 | 2.74 | 24 |
| A-blk | a-hosttoo | 2.01 [1.85-2.08] | 1.74 [1.66-1.80] | 2.89 [2.77-4.15] | 9.5 / 0.0 | 22 | 2.39 | 24 |
| A-blk | a-stock | 3.83 [3.76-3.92] | 2.61 [2.55-2.68] | 3.76 [3.69-3.93] | 15.6 / 0.1 | 262 | 2.73 | 24 |
| A-cb | a-hosttoo | 1.93 [1.87-2.12] | 1.72 [1.68-1.79] | 2.86 [2.55-4.63] | 7.4 / 0.0 | 21 | 2.39 | 24 |
| A-cb | a-stock | 3.84 [3.77-3.90] | 2.60 [2.58-2.67] | 3.75 [3.67-3.86] | 14.2 / 0.1 | 263 | 2.75 | 24 |
| A-q | a-hosttoo | 2.05 [1.92-2.13] | 1.75 [1.68-1.80] | 2.87 [2.66-4.78] | 7.2 / 0.0 | 21 | 2.38 | 24 |
| A-q | a-stock | 3.86 [3.82-3.91] | 2.62 [2.58-2.68] | 3.80 [3.74-3.87] | 14.0 / 0.1 | 262 | 2.74 | 24 |

### a: c54k1

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | a-hosttoo | 1.80 [1.78-2.03] | 1.66 [1.65-1.72] | 2.98 [2.91-3.04] | 10.1 / 0.0 | 19 | 2.25 | 24 |
| A | a-stock | 3.83 [3.77-3.92] | 2.59 [2.57-2.63] | 4.19 [4.11-4.28] | 11.1 / 0.1 | 258 | 2.89 | 24 |
| A-blk | a-hosttoo | 1.92 [1.82-1.98] | 1.70 [1.66-1.73] | 3.63 [3.25-4.36] | 10.0 / 0.0 | 21 | 2.87 | 24 |
| A-blk | a-stock | 3.73 [3.70-3.95] | 2.55 [2.52-2.66] | 4.15 [4.11-4.37] | 14.2 / 0.1 | 263 | 2.86 | 24 |
| A-cb | a-hosttoo | 1.89 [1.80-1.95] | 1.68 [1.64-1.71] | 3.14 [2.99-3.70] | 6.7 / 0.0 | 20 | 2.39 | 24 |
| A-cb | a-stock | 3.82 [3.72-3.87] | 2.60 [2.55-2.63] | 4.22 [4.13-4.27] | 12.7 / 0.1 | 262 | 2.89 | 24 |
| A-q | a-hosttoo | 2.00 [1.82-2.08] | 1.72 [1.66-1.78] | 3.10 [3.01-4.09] | 6.6 / 0.0 | 20 | 2.39 | 24 |
| A-q | a-stock | 3.85 [3.81-3.89] | 2.61 [2.58-2.63] | 4.25 [4.21-4.29] | 12.7 / 0.1 | 262 | 2.88 | 24 |

### a: c54k8

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | a-hosttoo | 2.06 [1.75-2.35] | 1.85 [1.60-2.02] | 2.79 [2.50-3.05] | 6.4 / 0.0 | 14 | 4.95 | 18 |
| A | a-stock | 3.68 [3.43-3.95] | 2.75 [2.57-3.00] | 3.59 [3.50-3.65] | 9.3 / 0.1 | 259 | 6.89 | 18 |
| A-blk | a-hosttoo | 1.89 [1.78-1.99] | 1.70 [1.67-1.73] | 2.79 [2.47-3.01] | 10.7 / 0.1 | 14 | 5.07 | 18 |
| A-blk | a-stock | 3.60 [2.50-3.75] | 2.67 [2.16-2.72] | 3.54 [2.95-3.68] | 14.0 / 0.1 | 259 | 6.64 | 18 |
| A-cb | a-hosttoo | 2.02 [1.94-2.56] | 1.86 [1.68-2.28] | 2.68 [2.51-2.95] | 5.7 / 0.0 | 13 | 4.91 | 18 |
| A-cb | a-stock | 3.79 [3.58-4.25] | 2.81 [2.62-3.23] | 3.55 [3.50-3.68] | 9.0 / 0.0 | 258 | 6.99 | 18 |
| A-q | a-hosttoo | 2.16 [1.93-2.41] | 1.86 [1.70-2.23] | 2.71 [2.54-2.98] | 6.2 / 0.0 | 13 | 5.01 | 18 |
| A-q | a-stock | 3.17 [2.46-3.80] | 2.50 [2.26-2.82] | 3.45 [2.91-3.58] | 10.0 / 0.0 | 259 | 6.63 | 18 |
## session cf

```
# in-process comparison: commit 99c18cd1 + UNCOMMITTED; 2026-10-01T21:35:55Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.53 2.24 2.29 1/940 1020084
# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts
# transport tcp (client target http://127.0.0.1:36375, TCP_NODELAY read back on every client socket); base env AK_PROBE_TASKCLOCK=1 AK_PROBE_SERVER_PID=1019772 AK_RPC_TARGET=http://127.0.0.1:36375 AK_EXPECT_NODELAY=1
# server also on TCP 127.0.0.1:36375 (pinned configuration, TCP_NODELAY on accept)
# netfilter (read-only): modules nft_log nft_limit xt_limit xt_NFLOG nfnetlink_log xt_physdev xt_multiport nf_conntrack_netlink xt_mark xt_nfacct nfnetlink_acct xt_comment xt_set ip_set xt_addrtype xt_CHECKSUM xt_MASQUERADE xt_conntrack ipt_REJECT nf_reject_ipv4 xt_tcpudp nft_compat nft_chain_nat nf_tables nfnetlink ip6_udp_tunnel nfit xt_nat br_netfilter nf_nat bridge nf_conntrack nf_defrag_ipv6 nf_defrag_ipv4 
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 1019772, pinned configuration, transport tcp
# cells A,Cf,Cf-cb,Cf-q (direction c: A,Cf,Cf-cb,Cf-q) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 c54k1 c54k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2; d16k16 5 x 1 batch of 16 warm 1; d16k32 4 x 1 x 32 warm 1; c54k16 6 x 2 x 16 warm 1; c54k32 5 x 2 x 32 warm 1); repetitions 3
# condition stock-w8: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-deliv/release/stream_probe (sha256 38f9fc7b27c4e040), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2batch-w8: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-deliv/release/stream_probe (sha256 38f9fc7b27c4e040), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w1: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-deliv/release/stream_probe (sha256 38f9fc7b27c4e040), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=1,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2batch-w1: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-deliv/release/stream_probe (sha256 38f9fc7b27c4e040), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=1,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 227 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.27 2.21 2.26 1/950 1025446
```


### cf: d16k1

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | h2batch-w1 | 14.89 [14.71-15.05] | 9.98 [9.93-10.06] | 14.59 [14.37-14.73] | 37.0 / 0.4 | 1035 | 11.03 | 24 |
| A | h2batch-w8 | 14.95 [14.85-15.20] | 9.99 [9.91-10.08] | 14.65 [14.52-14.82] | 35.5 / 0.4 | 1035 | 11.05 | 24 |
| A | stock-w1 | 15.01 [14.85-15.60] | 10.05 [9.92-10.57] | 14.63 [14.23-14.72] | 36.9 / 0.4 | 1036 | 11.04 | 24 |
| A | stock-w8 | 15.09 [14.93-15.22] | 10.05 [9.94-10.11] | 14.75 [14.50-14.91] | 36.9 / 0.4 | 1035 | 11.09 | 24 |
| Cf | h2batch-w1 | 7.82 [6.87-8.10] | 6.76 [6.29-6.91] | 8.73 [8.45-9.14] | 14.6 / 0.1 | 75 | 8.41 | 24 |
| Cf | h2batch-w8 | 7.98 [7.76-8.49] | 6.91 [6.80-7.57] | 9.14 [8.65-10.19] | 22.1 / 0.0 | 77 | 8.75 | 24 |
| Cf | stock-w1 | 14.70 [14.55-15.01] | 10.05 [9.98-10.22] | 13.00 [12.87-13.36] | 10.4 / 0.4 | 1028 | 10.54 | 24 |
| Cf | stock-w8 | 15.33 [15.18-15.70] | 10.68 [10.51-10.82] | 13.33 [13.22-13.66] | 45.9 / 0.2 | 1037 | 10.70 | 24 |
| Cf-cb | h2batch-w1 | 8.10 [7.57-8.21] | 6.92 [6.78-7.00] | 8.24 [8.00-9.75] | 28.0 / 0.1 | 78 | 8.70 | 24 |
| Cf-cb | h2batch-w8 | 7.82 [7.53-8.04] | 7.22 [6.98-7.43] | 9.34 [9.19-10.09] | 66.1 / 0.0 | 89 | 8.85 | 24 |
| Cf-cb | stock-w1 | 14.72 [14.43-14.91] | 10.15 [10.07-10.28] | 12.89 [12.63-13.10] | 21.5 / 0.4 | 1028 | 10.48 | 24 |
| Cf-cb | stock-w8 | 15.83 [15.62-16.60] | 11.04 [10.93-11.56] | 13.55 [13.35-14.27] | 76.8 / 0.2 | 1049 | 10.73 | 24 |
| Cf-q | h2batch-w1 | 7.90 [7.04-8.03] | 6.71 [6.40-6.80] | 8.85 [8.09-9.29] | 18.9 / 0.1 | 78 | 8.90 | 24 |
| Cf-q | h2batch-w8 | 7.93 [7.42-8.91] | 7.21 [6.95-7.64] | 9.04 [8.53-10.47] | 56.7 / 0.0 | 89 | 8.97 | 24 |
| Cf-q | stock-w1 | 14.53 [14.42-14.93] | 9.94 [9.86-10.24] | 12.83 [12.69-14.22] | 12.4 / 0.4 | 1028 | 10.41 | 24 |
| Cf-q | stock-w8 | 15.56 [15.37-15.79] | 10.84 [10.74-10.98] | 13.37 [13.27-13.57] | 68.8 / 0.2 | 1049 | 10.69 | 24 |

### cf: d16k8

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | h2batch-w1 | 14.92 [13.34-15.37] | 10.96 [10.16-12.13] | 12.94 [12.50-13.52] | 33.3 / 0.1 | 1033 | 21.38 | 18 |
| A | h2batch-w8 | 15.18 [9.52-15.71] | 11.06 [8.50-11.59] | 12.90 [9.21-13.24] | 33.0 / 0.1 | 1034 | 21.98 | 18 |
| A | stock-w1 | 15.39 [9.39-16.02] | 11.15 [8.39-11.88] | 13.02 [9.36-13.97] | 33.9 / 0.2 | 1034 | 20.40 | 18 |
| A | stock-w8 | 14.61 [9.34-16.65] | 10.71 [8.41-12.20] | 13.02 [9.21-13.55] | 32.3 / 0.2 | 1034 | 20.66 | 18 |
| Cf | h2batch-w1 | 8.37 [7.67-8.99] | 7.44 [7.16-7.82] | 8.53 [8.07-9.96] | 14.7 / 0.2 | 46 | 14.17 | 18 |
| Cf | h2batch-w8 | 8.24 [8.02-8.93] | 7.43 [7.33-7.70] | 8.36 [7.93-9.21] | 24.1 / 0.1 | 42 | 13.94 | 18 |
| Cf | stock-w1 | 15.49 [15.33-15.66] | 11.26 [11.14-11.40] | 13.08 [12.99-13.18] | 10.5 / 0.4 | 1033 | 21.23 | 18 |
| Cf | stock-w8 | 15.45 [15.11-15.75] | 11.33 [11.16-11.50] | 13.08 [12.93-13.22] | 40.7 / 0.2 | 1034 | 21.52 | 18 |
| Cf-cb | h2batch-w1 | 8.55 [8.16-8.95] | 7.54 [7.32-7.74] | 8.21 [7.80-8.63] | 19.9 / 0.1 | 47 | 14.00 | 18 |
| Cf-cb | h2batch-w8 | 8.67 [7.92-8.93] | 7.61 [7.23-7.76] | 8.20 [7.87-9.16] | 41.0 / 0.1 | 46 | 14.14 | 18 |
| Cf-cb | stock-w1 | 15.30 [14.78-15.76] | 11.19 [10.91-11.33] | 12.98 [12.90-13.58] | 14.4 / 0.4 | 1033 | 21.52 | 18 |
| Cf-cb | stock-w8 | 15.33 [14.49-15.61] | 11.23 [10.95-11.42] | 12.99 [12.88-13.22] | 55.8 / 0.3 | 1039 | 21.60 | 18 |
| Cf-q | h2batch-w1 | 6.38 [6.33-6.43] | 5.97 [5.91-6.01] | 9.36 [9.07-9.66] | 28.2 / 0.0 | 70 | 13.58 | 18 |
| Cf-q | h2batch-w8 | 7.24 [6.78-7.90] | 6.47 [6.33-6.84] | 8.31 [8.08-10.28] | 40.7 / 0.0 | 49 | 13.78 | 18 |
| Cf-q | stock-w1 | 14.08 [7.92-14.26] | 9.95 [7.34-10.08] | 12.78 [9.16-12.95] | 5.8 / 0.3 | 1032 | 20.01 | 18 |
| Cf-q | stock-w8 | 14.69 [14.58-15.04] | 10.61 [10.57-10.91] | 12.97 [12.83-13.16] | 55.5 / 0.2 | 1040 | 22.11 | 18 |

### cf: d4k1

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | h2batch-w1 | 3.80 [3.75-3.85] | 2.58 [2.56-2.62] | 3.69 [3.64-3.75] | 13.0 / 0.1 | 261 | 2.74 | 24 |
| A | h2batch-w8 | 3.83 [3.79-3.95] | 2.60 [2.56-2.68] | 3.70 [3.64-3.81] | 12.9 / 0.1 | 261 | 2.76 | 24 |
| A | stock-w1 | 3.83 [3.77-3.94] | 2.58 [2.56-2.71] | 3.72 [3.65-3.83] | 13.0 / 0.1 | 261 | 2.75 | 24 |
| A | stock-w8 | 3.83 [3.72-3.92] | 2.59 [2.55-2.63] | 3.69 [3.61-3.75] | 13.0 / 0.1 | 261 | 2.75 | 24 |
| Cf | h2batch-w1 | 1.72 [1.68-1.82] | 1.56 [1.51-1.62] | 2.53 [2.50-3.11] | 5.2 / 0.0 | 21 | 2.24 | 24 |
| Cf | h2batch-w8 | 2.02 [1.88-2.09] | 1.79 [1.73-1.84] | 2.63 [2.57-2.74] | 13.2 / 0.0 | 23 | 2.22 | 24 |
| Cf | stock-w1 | 3.59 [3.56-3.67] | 2.43 [2.41-2.49] | 3.53 [3.51-3.60] | 4.1 / 0.1 | 260 | 2.69 | 24 |
| Cf | stock-w8 | 3.84 [3.79-3.88] | 2.66 [2.64-2.70] | 3.67 [3.63-3.70] | 17.1 / 0.1 | 262 | 2.73 | 24 |
| Cf-cb | h2batch-w1 | 1.80 [1.73-1.83] | 1.64 [1.60-1.66] | 2.54 [2.40-2.76] | 11.3 / 0.0 | 21 | 2.16 | 24 |
| Cf-cb | h2batch-w8 | 2.15 [1.93-2.22] | 1.90 [1.79-1.95] | 2.58 [2.52-2.81] | 22.2 / 0.0 | 26 | 2.21 | 24 |
| Cf-cb | stock-w1 | 3.72 [3.63-3.81] | 2.53 [2.49-2.57] | 3.62 [3.52-3.70] | 9.2 / 0.1 | 260 | 2.71 | 24 |
| Cf-cb | stock-w8 | 3.95 [3.90-4.04] | 2.74 [2.72-2.81] | 3.68 [3.63-3.74] | 28.2 / 0.1 | 266 | 2.73 | 24 |
| Cf-q | h2batch-w1 | 1.89 [1.69-2.14] | 1.66 [1.55-1.81] | 2.59 [2.43-2.71] | 7.6 / 0.0 | 21 | 2.19 | 24 |
| Cf-q | h2batch-w8 | 1.89 [1.85-2.18] | 1.75 [1.70-1.88] | 2.56 [2.50-2.78] | 20.5 / 0.0 | 25 | 2.21 | 24 |
| Cf-q | stock-w1 | 3.64 [3.59-3.75] | 2.48 [2.45-2.54] | 3.56 [3.51-3.65] | 6.1 / 0.1 | 260 | 2.70 | 24 |
| Cf-q | stock-w8 | 3.91 [3.83-3.96] | 2.74 [2.70-2.79] | 3.70 [3.64-3.75] | 25.6 / 0.1 | 265 | 2.73 | 24 |

### cf: c54k1

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | h2batch-w1 | 3.73 [3.69-3.86] | 2.55 [2.53-2.70] | 4.06 [4.02-4.20] | 11.1 / 0.1 | 258 | 2.84 | 24 |
| A | h2batch-w8 | 3.79 [3.73-3.93] | 2.60 [2.55-2.71] | 4.15 [4.07-4.48] | 11.1 / 0.1 | 258 | 2.89 | 24 |
| A | stock-w1 | 3.79 [3.71-3.87] | 2.58 [2.56-2.64] | 4.13 [4.05-4.21] | 11.2 / 0.1 | 258 | 2.87 | 24 |
| A | stock-w8 | 3.76 [3.72-3.88] | 2.56 [2.54-2.62] | 4.10 [4.06-4.19] | 11.1 / 0.1 | 258 | 2.86 | 24 |
| Cf | h2batch-w1 | 1.77 [1.72-1.88] | 1.61 [1.58-1.66] | 3.72 [2.97-6.15] | 7.3 / 0.0 | 20 | 2.93 | 24 |
| Cf | h2batch-w8 | 1.91 [1.81-2.00] | 1.71 [1.68-1.76] | 3.25 [3.07-3.77] | 11.2 / 0.0 | 21 | 2.36 | 24 |
| Cf | stock-w1 | 3.74 [3.68-3.80] | 2.52 [2.50-2.54] | 4.22 [4.18-4.29] | 5.1 / 0.1 | 260 | 2.86 | 24 |
| Cf | stock-w8 | 3.88 [3.85-3.93] | 2.66 [2.65-2.71] | 4.26 [4.24-4.31] | 15.2 / 0.1 | 260 | 2.87 | 24 |
| Cf-cb | h2batch-w1 | 1.79 [1.74-1.89] | 1.61 [1.59-1.67] | 4.67 [3.46-5.50] | 5.5 / 0.0 | 19 | 3.96 | 24 |
| Cf-cb | h2batch-w8 | 1.89 [1.82-2.13] | 1.73 [1.69-1.80] | 3.18 [3.01-3.79] | 11.1 / 0.0 | 20 | 2.32 | 24 |
| Cf-cb | stock-w1 | 3.79 [3.74-4.09] | 2.54 [2.51-2.70] | 4.28 [4.24-4.55] | 4.2 / 0.1 | 259 | 2.87 | 24 |
| Cf-cb | stock-w8 | 3.89 [3.86-3.94] | 2.66 [2.65-2.69] | 4.27 [4.24-4.33] | 14.1 / 0.1 | 260 | 2.86 | 24 |
| Cf-q | h2batch-w1 | 1.74 [1.70-2.02] | 1.59 [1.56-1.70] | 3.08 [2.94-3.25] | 5.2 / 0.0 | 19 | 2.25 | 24 |
| Cf-q | h2batch-w8 | 2.03 [1.77-2.07] | 1.74 [1.64-1.76] | 3.16 [3.04-4.11] | 7.7 / 0.0 | 20 | 2.30 | 24 |
| Cf-q | stock-w1 | 3.75 [3.69-3.82] | 2.50 [2.47-2.53] | 4.25 [4.21-4.70] | 3.1 / 0.1 | 259 | 2.87 | 24 |
| Cf-q | stock-w8 | 3.86 [3.83-4.14] | 2.64 [2.62-2.81] | 4.26 [4.19-4.50] | 12.7 / 0.1 | 260 | 2.87 | 24 |

### cf: c54k8

| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|
| A | h2batch-w1 | 3.34 [2.78-3.98] | 2.65 [2.25-3.01] | 3.46 [3.02-3.63] | 9.3 / 0.0 | 259 | 6.36 | 18 |
| A | h2batch-w8 | 3.67 [3.13-4.01] | 2.78 [2.47-3.19] | 3.52 [3.44-3.78] | 9.2 / 0.0 | 259 | 6.63 | 18 |
| A | stock-w1 | 3.73 [3.49-4.00] | 2.79 [2.62-3.08] | 3.55 [3.45-3.63] | 9.4 / 0.0 | 258 | 6.89 | 18 |
| A | stock-w8 | 3.65 [3.39-3.94] | 2.76 [2.55-3.06] | 3.54 [3.51-3.70] | 9.3 / 0.0 | 259 | 6.69 | 18 |
| Cf | h2batch-w1 | 2.21 [2.14-2.29] | 2.00 [1.94-2.05] | 2.81 [2.68-2.94] | 6.4 / 0.0 | 12 | 5.34 | 18 |
| Cf | h2batch-w8 | 2.22 [2.16-2.24] | 2.08 [2.02-2.12] | 3.08 [2.86-3.19] | 13.9 / 0.0 | 18 | 5.28 | 18 |
| Cf | stock-w1 | 3.80 [2.77-3.93] | 2.93 [2.46-3.01] | 3.57 [2.95-3.67] | 5.5 / 0.1 | 258 | 7.29 | 18 |
| Cf | stock-w8 | 4.05 [2.87-4.17] | 3.09 [2.60-3.19] | 3.58 [3.11-3.67] | 13.5 / 0.0 | 258 | 7.14 | 18 |
| Cf-cb | h2batch-w1 | 1.92 [1.84-2.05] | 1.64 [1.60-1.76] | 2.75 [2.47-3.11] | 6.0 / 0.0 | 13 | 5.07 | 18 |
| Cf-cb | h2batch-w8 | 1.93 [1.85-1.99] | 1.72 [1.69-1.83] | 2.87 [2.68-3.43] | 11.4 / 0.0 | 13 | 5.53 | 18 |
| Cf-cb | stock-w1 | 3.41 [3.03-3.52] | 2.49 [2.34-2.55] | 3.55 [3.41-3.66] | 5.0 / 0.1 | 257 | 6.81 | 18 |
| Cf-cb | stock-w8 | 3.63 [3.35-3.72] | 2.70 [2.58-2.74] | 3.62 [3.51-3.70] | 14.1 / 0.0 | 259 | 6.88 | 18 |
| Cf-q | h2batch-w1 | 1.75 [1.68-1.79] | 1.53 [1.52-1.55] | 2.77 [2.68-2.95] | 2.8 / 0.0 | 13 | 5.05 | 18 |
| Cf-q | h2batch-w8 | 1.87 [1.81-2.03] | 1.65 [1.60-1.72] | 2.73 [2.56-2.87] | 7.7 / 0.0 | 13 | 5.31 | 18 |
| Cf-q | stock-w1 | 3.27 [2.32-3.40] | 2.41 [1.96-2.46] | 3.53 [3.06-3.68] | 1.8 / 0.1 | 257 | 6.69 | 18 |
| Cf-q | stock-w8 | 3.60 [3.24-3.67] | 2.62 [2.49-2.70] | 3.59 [3.46-3.67] | 10.4 / 0.0 | 259 | 6.99 | 18 |
