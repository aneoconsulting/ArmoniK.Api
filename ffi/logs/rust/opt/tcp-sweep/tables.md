# TCP core worker sweep (gen/tcp_sweep.sh)

## header (low)

```
# in-process comparison: commit ca203a97 + UNCOMMITTED; 2026-10-01T20:59:07Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 1.57 1.52 0.94 1/942 962634
# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts
# transport tcp (client target http://127.0.0.1:46769, TCP_NODELAY read back on every client socket); base env AK_PROBE_TASKCLOCK=1 AK_PROBE_SERVER_PID=962313 AK_RPC_TARGET=http://127.0.0.1:46769 AK_EXPECT_NODELAY=1
# server also on TCP 127.0.0.1:46769 (pinned configuration, TCP_NODELAY on accept)
# netfilter (read-only): modules nft_log nft_limit xt_limit xt_NFLOG nfnetlink_log xt_physdev xt_multiport nf_conntrack_netlink xt_mark xt_nfacct nfnetlink_acct xt_comment xt_set ip_set xt_addrtype xt_CHECKSUM xt_MASQUERADE xt_conntrack ipt_REJECT nf_reject_ipv4 xt_tcpudp nft_compat nft_chain_nat nf_tables nfnetlink ip6_udp_tunnel nfit xt_nat br_netfilter nf_nat bridge nf_conntrack nf_defrag_ipv6 nf_defrag_ipv4 
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 962313, pinned configuration, transport tcp
# cells A,Cf,Cf-cb (direction c: A,Cf,Cf-cb) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 c54k1 c54k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2; d16k16 5 x 1 batch of 16 warm 1; d16k32 4 x 1 x 32 warm 1; c54k16 6 x 2 x 16 warm 1; c54k32 5 x 2 x 32 warm 1); repetitions 3
# condition stock-w1: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=1,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w1: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=1,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w2: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=2,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w2: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=2,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w4: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=4,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w4: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=4,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w8: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w8: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 308 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.13 2.02 1.34 1/947 970722
```

## header (high)

```
# in-process comparison: commit 7d83c872 + UNCOMMITTED; 2026-10-01T21:06:48Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 4.15 3.06 1.84 2/940 975752
# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts
# transport tcp (client target http://127.0.0.1:43135, TCP_NODELAY read back on every client socket); base env AK_PROBE_TASKCLOCK=1 AK_PROBE_SERVER_PID=975183 AK_RPC_TARGET=http://127.0.0.1:43135 AK_EXPECT_NODELAY=1
# server also on TCP 127.0.0.1:43135 (pinned configuration, TCP_NODELAY on accept)
# netfilter (read-only): modules nft_log nft_limit xt_limit xt_NFLOG nfnetlink_log xt_physdev xt_multiport nf_conntrack_netlink xt_mark xt_nfacct nfnetlink_acct xt_comment xt_set ip_set xt_addrtype xt_CHECKSUM xt_MASQUERADE xt_conntrack ipt_REJECT nf_reject_ipv4 xt_tcpudp nft_compat nft_chain_nat nf_tables nfnetlink ip6_udp_tunnel nfit xt_nat br_netfilter nf_nat bridge nf_conntrack nf_defrag_ipv6 nf_defrag_ipv4 
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 975183, pinned configuration, transport tcp
# cells A,Cf,Cf-cb,Cf-m4,Cf-cb-m4 (direction c: A,Cf,Cf-cb,Cf-m4,Cf-cb-m4) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k16 d16k32 c54k16 c54k32 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2; d16k16 5 x 1 batch of 16 warm 1; d16k32 4 x 1 x 32 warm 1; c54k16 6 x 2 x 16 warm 1; c54k32 5 x 2 x 32 warm 1); repetitions 3
# condition stock-w1: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=1,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w1: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=1,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w2: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=2,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w2: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=2,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w4: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=4,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w4: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=4,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w8: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w8: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 578 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x2, 0,9-10,19 x47, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 4.18 3.64 2.62 1/945 994454
```

## Notes

- Run after the owner's re-pin following the second suspend (19:53:03 to 20:36:33 local), verified by the coordinator and here before the run: 47 IRQs on 0,9-10,19 and 2 on 0-19; sleep inhibitor 'sleep infinity' (block, sleep) in place. Same driver, cores and conditions as ../tcp-sweep-unpinned-irqs/.

CPU = client task-clock; P5.4 throughput in calls/s only.


## d16k1

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 14.87 [14.76-15.00] | 9.98 [9.93-10.04] | 14.54 [14.43-14.71] | 69 | 1154 | 36.9 / 0.4 | 1035 | 11.01 | 24 |
| A | stock | 2 | 14.99 [14.84-15.16] | 10.05 [9.99-10.10] | 14.67 [14.51-14.84] | 68 | 1143 | 36.5 / 0.4 | 1035 | 11.08 | 24 |
| A | stock | 4 | 15.00 [14.90-15.15] | 10.02 [9.92-10.13] | 14.63 [14.49-14.82] | 68 | 1147 | 37.0 / 0.4 | 1035 | 11.01 | 24 |
| A | stock | 8 | 14.83 [14.67-15.05] | 9.98 [9.79-10.07] | 14.49 [14.35-14.65] | 69 | 1158 | 37.0 / 0.4 | 1035 | 10.96 | 24 |
| A | h2-batch | 1 | 14.90 [14.70-15.11] | 9.98 [9.89-10.07] | 14.63 [14.43-14.80] | 68 | 1147 | 36.9 / 0.4 | 1035 | 10.98 | 24 |
| A | h2-batch | 2 | 14.96 [14.75-15.48] | 10.01 [9.90-10.39] | 14.59 [14.39-14.74] | 69 | 1150 | 36.9 / 0.4 | 1035 | 10.96 | 24 |
| A | h2-batch | 4 | 14.98 [14.82-15.13] | 10.03 [9.88-10.14] | 14.68 [14.52-14.88] | 68 | 1143 | 35.4 / 0.4 | 1035 | 11.09 | 24 |
| A | h2-batch | 8 | 14.95 [14.86-15.06] | 10.04 [9.97-10.09] | 14.64 [14.53-14.72] | 68 | 1146 | 36.9 / 0.4 | 1035 | 11.02 | 24 |
| Cf | stock | 1 | 14.63 [9.02-14.87] | 9.99 [7.86-10.11] | 12.96 [9.22-13.20] | 77 | 1295 | 10.2 / 0.3 | 1028 | 10.47 | 24 |
| Cf | stock | 2 | 15.25 [15.14-15.74] | 10.47 [10.44-10.61] | 13.30 [13.20-13.72] | 75 | 1262 | 40.1 / 0.4 | 1042 | 10.61 | 24 |
| Cf | stock | 4 | 15.23 [15.03-15.43] | 10.54 [10.45-10.66] | 13.29 [13.14-13.45] | 75 | 1262 | 44.8 / 0.2 | 1036 | 10.64 | 24 |
| Cf | stock | 8 | 15.26 [15.15-15.53] | 10.53 [10.49-10.71] | 13.36 [13.21-13.55] | 75 | 1256 | 44.7 / 0.2 | 1036 | 10.67 | 24 |
| Cf | h2-batch | 1 | 7.00 [6.91-7.81] | 6.49 [6.39-6.70] | 9.64 [8.50-10.28] | 104 | 1741 | 25.8 / 0.0 | 79 | 8.70 | 24 |
| Cf | h2-batch | 2 | 7.51 [7.12-7.85] | 6.68 [6.54-6.82] | 8.93 [8.70-10.88] | 112 | 1880 | 19.9 / 0.0 | 79 | 9.00 | 24 |
| Cf | h2-batch | 4 | 8.22 [7.81-8.85] | 7.03 [6.83-7.55] | 8.76 [8.40-10.09] | 114 | 1916 | 23.6 / 0.0 | 77 | 8.56 | 24 |
| Cf | h2-batch | 8 | 7.77 [7.24-8.39] | 6.79 [6.58-7.15] | 8.79 [8.54-10.08] | 114 | 1908 | 22.0 / 0.0 | 77 | 8.75 | 24 |
| Cf-cb | stock | 1 | 14.68 [14.55-14.93] | 10.15 [10.08-10.26] | 12.86 [12.71-13.11] | 78 | 1305 | 21.4 / 0.4 | 1028 | 10.44 | 24 |
| Cf-cb | stock | 2 | 15.42 [9.96-15.56] | 10.71 [8.79-10.80] | 13.26 [9.63-13.41] | 75 | 1265 | 62.8 / 0.4 | 1053 | 10.61 | 24 |
| Cf-cb | stock | 4 | 15.76 [15.59-15.92] | 11.00 [10.88-11.09] | 13.50 [13.34-13.66] | 74 | 1242 | 74.7 / 0.4 | 1048 | 10.70 | 24 |
| Cf-cb | stock | 8 | 15.78 [15.64-15.90] | 11.01 [10.95-11.09] | 13.51 [13.40-13.61] | 74 | 1242 | 78.6 / 0.2 | 1048 | 10.68 | 24 |
| Cf-cb | h2-batch | 1 | 7.11 [6.97-8.55] | 6.59 [6.53-7.41] | 9.30 [8.82-10.15] | 108 | 1805 | 34.8 / 0.0 | 80 | 8.72 | 24 |
| Cf-cb | h2-batch | 2 | 8.22 [7.40-8.36] | 7.19 [6.85-7.31] | 8.83 [8.61-9.29] | 113 | 1901 | 47.6 / 0.0 | 89 | 8.51 | 24 |
| Cf-cb | h2-batch | 4 | 8.00 [7.78-8.84] | 7.40 [7.25-7.73] | 9.24 [8.51-10.64] | 108 | 1815 | 63.6 / 0.0 | 89 | 8.98 | 24 |
| Cf-cb | h2-batch | 8 | 8.06 [7.60-8.63] | 7.25 [7.04-7.58] | 8.97 [8.59-10.64] | 111 | 1870 | 59.1 / 0.0 | 87 | 9.02 | 24 |

## d16k8

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 14.86 [9.00-15.44] | 10.94 [8.22-11.57] | 13.01 [9.12-13.11] | 77 | 1290 | 32.1 / 0.1 | 1034 | 21.38 | 18 |
| A | stock | 2 | 15.21 [9.01-15.93] | 11.11 [8.32-11.61] | 12.94 [10.01-13.13] | 77 | 1297 | 35.1 / 0.2 | 1034 | 20.64 | 18 |
| A | stock | 4 | 15.25 [15.06-15.87] | 11.03 [10.90-11.79] | 12.94 [12.83-13.04] | 77 | 1297 | 34.8 / 0.1 | 1033 | 21.82 | 18 |
| A | stock | 8 | 15.17 [9.45-15.60] | 11.06 [8.40-11.47] | 12.89 [9.26-13.11] | 78 | 1302 | 32.4 / 0.2 | 1034 | 21.19 | 18 |
| A | h2-batch | 1 | 14.88 [9.22-15.71] | 10.86 [8.40-11.65] | 12.89 [9.88-13.07] | 78 | 1302 | 33.5 / 0.1 | 1034 | 21.80 | 18 |
| A | h2-batch | 2 | 13.87 [9.13-15.82] | 10.41 [8.22-12.08] | 12.64 [9.25-13.03] | 79 | 1327 | 30.8 / 0.1 | 1033 | 20.48 | 18 |
| A | h2-batch | 4 | 14.02 [9.10-15.33] | 10.41 [8.26-11.51] | 12.55 [9.10-12.98] | 80 | 1337 | 30.7 / 0.1 | 1034 | 19.56 | 18 |
| A | h2-batch | 8 | 15.19 [14.33-16.27] | 10.97 [10.78-12.08] | 13.03 [12.87-13.20] | 77 | 1287 | 33.2 / 0.2 | 1033 | 21.07 | 18 |
| Cf | stock | 1 | 15.44 [9.60-15.64] | 11.26 [8.69-11.39] | 13.02 [9.21-13.12] | 77 | 1288 | 10.7 / 0.4 | 1034 | 21.06 | 18 |
| Cf | stock | 2 | 15.60 [9.36-15.80] | 11.43 [8.64-11.57] | 12.99 [9.77-13.07] | 77 | 1292 | 33.8 / 0.3 | 1038 | 21.17 | 18 |
| Cf | stock | 4 | 15.47 [15.35-15.60] | 11.36 [11.23-11.49] | 13.03 [12.94-13.13] | 77 | 1287 | 39.4 / 0.2 | 1034 | 21.89 | 18 |
| Cf | stock | 8 | 15.57 [15.45-15.76] | 11.51 [11.41-11.58] | 12.99 [12.91-13.18] | 77 | 1291 | 40.1 / 0.2 | 1035 | 22.38 | 18 |
| Cf | h2-batch | 1 | 8.45 [8.27-8.81] | 7.58 [7.43-7.79] | 8.39 [8.09-8.71] | 119 | 1999 | 14.5 / 0.2 | 47 | 14.14 | 18 |
| Cf | h2-batch | 2 | 8.54 [8.24-8.85] | 7.55 [7.35-7.71] | 8.25 [8.05-8.39] | 121 | 2033 | 19.7 / 0.1 | 43 | 13.88 | 18 |
| Cf | h2-batch | 4 | 8.89 [8.73-9.02] | 7.66 [7.55-7.78] | 7.94 [7.61-8.45] | 126 | 2113 | 23.2 / 0.1 | 41 | 13.84 | 18 |
| Cf | h2-batch | 8 | 8.39 [7.96-8.85] | 7.47 [7.35-7.69] | 8.41 [7.96-9.99] | 119 | 1996 | 24.0 / 0.1 | 42 | 13.79 | 18 |
| Cf-cb | stock | 1 | 15.37 [9.50-15.52] | 11.14 [8.62-11.29] | 13.02 [9.17-13.20] | 77 | 1288 | 14.4 / 0.4 | 1033 | 20.47 | 18 |
| Cf-cb | stock | 2 | 15.40 [15.15-15.46] | 11.21 [11.09-11.42] | 13.06 [12.92-13.18] | 77 | 1285 | 43.8 / 0.3 | 1045 | 22.08 | 18 |
| Cf-cb | stock | 4 | 15.50 [10.83-15.68] | 11.36 [9.37-11.53] | 12.96 [10.00-13.31] | 77 | 1294 | 55.0 / 0.3 | 1041 | 21.58 | 18 |
| Cf-cb | stock | 8 | 14.73 [9.41-15.24] | 11.08 [8.60-11.29] | 12.79 [9.13-13.07] | 78 | 1311 | 54.1 / 0.2 | 1040 | 21.21 | 18 |
| Cf-cb | h2-batch | 1 | 8.26 [7.74-8.44] | 7.38 [7.21-7.55] | 8.49 [8.21-10.97] | 118 | 1977 | 21.1 / 0.1 | 48 | 13.92 | 18 |
| Cf-cb | h2-batch | 2 | 8.73 [8.49-8.95] | 7.65 [7.47-7.82] | 7.93 [7.63-8.34] | 126 | 2115 | 32.2 / 0.1 | 49 | 13.83 | 18 |
| Cf-cb | h2-batch | 4 | 8.85 [8.31-9.11] | 7.72 [7.50-7.92] | 8.08 [7.92-8.62] | 124 | 2076 | 40.9 / 0.1 | 46 | 14.17 | 18 |
| Cf-cb | h2-batch | 8 | 8.78 [8.14-9.17] | 7.75 [7.55-7.96] | 8.21 [7.78-9.31] | 122 | 2044 | 41.3 / 0.1 | 46 | 13.90 | 18 |

## d16k16

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 15.21 [14.52-17.62] | 11.08 [10.77-13.49] | 13.01 [12.87-13.61] | 77 | 1289 | 32.9 / 0.2 | 1034 | 22.06 | 15 |
| A | stock | 2 | 15.60 [14.57-17.46] | 11.50 [10.75-13.27] | 13.06 [12.85-13.31] | 77 | 1284 | 33.7 / 0.2 | 1034 | 21.92 | 15 |
| A | stock | 4 | 15.38 [14.30-17.67] | 11.73 [10.75-13.14] | 12.90 [12.70-13.31] | 78 | 1300 | 32.7 / 0.2 | 1034 | 22.36 | 15 |
| A | stock | 8 | 15.49 [14.95-17.20] | 11.33 [10.95-13.00] | 13.12 [12.89-13.34] | 76 | 1279 | 33.0 / 0.2 | 1034 | 22.30 | 15 |
| A | h2-batch | 1 | 15.22 [8.90-16.56] | 11.24 [8.19-12.78] | 12.86 [9.22-13.05] | 78 | 1305 | 32.8 / 0.1 | 1034 | 21.68 | 15 |
| A | h2-batch | 2 | 15.70 [12.93-17.22] | 11.50 [10.01-13.12] | 12.96 [12.40-13.56] | 77 | 1295 | 33.0 / 0.2 | 1034 | 21.90 | 15 |
| A | h2-batch | 4 | 14.68 [9.76-17.03] | 11.02 [8.94-12.68] | 12.93 [9.39-13.73] | 77 | 1297 | 31.4 / 0.2 | 1034 | 21.60 | 15 |
| A | h2-batch | 8 | 15.15 [13.59-17.68] | 11.52 [10.63-13.16] | 12.82 [12.29-13.36] | 78 | 1309 | 31.2 / 0.2 | 1034 | 21.84 | 15 |
| Cf | stock | 1 | 15.03 [8.85-15.64] | 10.95 [8.26-11.51] | 12.92 [8.67-13.03] | 77 | 1298 | 10.8 / 0.4 | 1035 | 20.89 | 15 |
| Cf | stock | 2 | 14.68 [9.16-15.46] | 10.94 [8.52-11.31] | 12.75 [9.08-13.08] | 78 | 1316 | 30.4 / 0.4 | 1038 | 21.33 | 15 |
| Cf | stock | 4 | 15.33 [12.14-15.81] | 11.24 [9.90-11.58] | 12.90 [11.42-13.19] | 78 | 1301 | 37.8 / 0.2 | 1036 | 22.24 | 15 |
| Cf | stock | 8 | 15.27 [9.38-15.76] | 11.34 [8.64-11.54] | 12.93 [9.53-13.07] | 77 | 1297 | 37.9 / 0.3 | 1036 | 22.38 | 15 |
| Cf | h2-batch | 1 | 7.75 [7.54-8.09] | 7.20 [7.02-7.34] | 9.12 [8.38-9.34] | 110 | 1840 | 13.7 / 0.2 | 55 | 13.82 | 15 |
| Cf | h2-batch | 2 | 7.92 [7.58-8.19] | 7.37 [7.03-7.69] | 10.50 [9.19-11.28] | 95 | 1598 | 36.2 / 0.1 | 64 | 13.04 | 15 |
| Cf | h2-batch | 4 | 8.69 [7.94-9.10] | 7.60 [7.28-7.86] | 8.23 [7.79-9.02] | 122 | 2040 | 23.4 / 0.2 | 40 | 14.10 | 15 |
| Cf | h2-batch | 8 | 8.93 [8.74-9.00] | 7.77 [7.61-7.99] | 8.19 [7.80-8.60] | 122 | 2049 | 22.9 / 0.2 | 40 | 14.29 | 15 |
| Cf-cb | stock | 1 | 15.51 [15.16-15.74] | 11.30 [10.95-11.53] | 12.94 [12.82-13.15] | 77 | 1296 | 12.1 / 0.4 | 1032 | 21.37 | 15 |
| Cf-cb | stock | 2 | 15.37 [14.51-15.57] | 11.27 [10.85-11.49] | 12.83 [12.67-13.00] | 78 | 1308 | 40.9 / 0.4 | 1045 | 22.55 | 15 |
| Cf-cb | stock | 4 | 15.45 [9.33-15.94] | 11.32 [8.61-11.81] | 12.99 [9.04-13.29] | 77 | 1291 | 54.1 / 0.2 | 1041 | 21.03 | 15 |
| Cf-cb | stock | 8 | 14.25 [9.32-15.59] | 10.87 [8.72-11.58] | 12.65 [9.88-13.17] | 79 | 1326 | 50.7 / 0.2 | 1041 | 21.86 | 15 |
| Cf-cb | h2-batch | 1 | 8.68 [8.43-8.92] | 7.66 [7.48-7.88] | 8.23 [7.79-8.48] | 122 | 2040 | 18.1 / 0.1 | 45 | 14.21 | 15 |
| Cf-cb | h2-batch | 2 | 8.88 [8.53-9.09] | 7.75 [7.62-8.10] | 8.18 [7.94-8.50] | 122 | 2051 | 31.0 / 0.1 | 49 | 14.17 | 15 |
| Cf-cb | h2-batch | 4 | 9.06 [8.47-9.24] | 7.89 [7.67-8.13] | 8.07 [7.86-8.49] | 124 | 2079 | 37.4 / 0.1 | 44 | 14.05 | 15 |
| Cf-cb | h2-batch | 8 | 8.67 [7.91-9.02] | 7.70 [7.29-8.07] | 8.34 [7.86-9.63] | 120 | 2011 | 38.7 / 0.2 | 45 | 13.99 | 15 |
| Cf-m4 | stock | 1 | 17.30 [15.34-19.83] | 13.27 [12.41-14.68] | 4.00 [3.86-4.97] | 250 | 4190 | 12.7 / 0.4 | 1033 | 23.52 | 15 |
| Cf-m4 | stock | 2 | 20.88 [19.09-21.64] | 15.19 [14.24-15.70] | 4.77 [4.61-4.96] | 210 | 3516 | 38.8 / 0.5 | 1036 | 23.72 | 15 |
| Cf-m4 | stock | 4 | 20.37 [19.50-21.55] | 15.02 [14.55-15.79] | 4.82 [4.67-5.00] | 208 | 3483 | 45.2 / 0.8 | 1032 | 23.74 | 15 |
| Cf-m4 | stock | 8 | 20.50 [19.51-21.46] | 15.11 [14.52-15.63] | 4.79 [4.54-4.95] | 209 | 3506 | 46.4 / 0.8 | 1032 | 23.55 | 15 |
| Cf-m4 | h2-batch | 1 | 12.11 [11.41-12.42] | 10.98 [10.16-11.20] | 3.82 [3.71-3.91] | 262 | 4393 | 21.3 / 0.4 | 57 | 20.00 | 15 |
| Cf-m4 | h2-batch | 2 | 12.57 [11.98-12.97] | 11.36 [10.71-11.61] | 3.65 [3.54-3.99] | 274 | 4593 | 21.9 / 0.6 | 48 | 20.64 | 15 |
| Cf-m4 | h2-batch | 4 | 12.15 [11.74-12.87] | 10.99 [10.65-11.70] | 3.76 [3.56-3.97] | 266 | 4461 | 30.2 / 0.6 | 54 | 19.93 | 15 |
| Cf-m4 | h2-batch | 8 | 12.33 [11.72-12.80] | 11.14 [10.76-11.60] | 3.97 [3.52-4.25] | 252 | 4226 | 27.5 / 0.6 | 50 | 20.42 | 15 |
| Cf-cb-m4 | stock | 1 | 19.67 [18.00-22.14] | 14.59 [13.70-16.37] | 4.77 [4.01-5.04] | 210 | 3516 | 13.2 / 0.4 | 1030 | 23.47 | 15 |
| Cf-cb-m4 | stock | 2 | 20.31 [18.66-22.13] | 15.24 [14.25-16.43] | 4.73 [4.61-4.92] | 212 | 3550 | 47.1 / 0.8 | 1043 | 23.49 | 15 |
| Cf-cb-m4 | stock | 4 | 20.75 [18.36-21.74] | 15.19 [14.10-15.87] | 4.74 [4.51-4.96] | 211 | 3540 | 60.2 / 1.5 | 1034 | 23.72 | 15 |
| Cf-cb-m4 | stock | 8 | 19.06 [17.28-20.96] | 14.61 [13.70-15.64] | 4.70 [4.24-4.77] | 213 | 3569 | 60.1 / 1.4 | 1037 | 23.50 | 15 |
| Cf-cb-m4 | h2-batch | 1 | 12.46 [11.79-12.96] | 11.01 [10.66-11.66] | 3.63 [3.50-4.16] | 275 | 4617 | 19.7 / 0.3 | 52 | 20.30 | 15 |
| Cf-cb-m4 | h2-batch | 2 | 13.09 [12.47-14.18] | 11.69 [10.99-12.97] | 3.62 [3.46-3.88] | 276 | 4638 | 32.9 / 0.9 | 55 | 20.64 | 15 |
| Cf-cb-m4 | h2-batch | 4 | 13.15 [12.61-14.65] | 11.81 [11.18-13.30] | 3.62 [3.51-3.96] | 276 | 4633 | 41.1 / 1.1 | 51 | 20.55 | 15 |
| Cf-cb-m4 | h2-batch | 8 | 13.37 [13.00-14.18] | 11.97 [11.56-12.92] | 3.48 [3.40-3.81] | 287 | 4819 | 39.4 / 1.2 | 50 | 20.69 | 15 |

## d16k32

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 15.34 [14.96-17.45] | 11.27 [10.90-13.03] | 13.04 [12.94-13.54] | 77 | 1287 | 33.4 / 0.2 | 1034 | 22.87 | 12 |
| A | stock | 2 | 15.78 [15.14-16.11] | 11.61 [10.99-11.94] | 13.15 [13.05-13.29] | 76 | 1276 | 33.7 / 0.2 | 1034 | 22.75 | 12 |
| A | stock | 4 | 15.44 [9.26-16.01] | 11.30 [8.46-11.79] | 12.97 [9.31-13.07] | 77 | 1293 | 32.5 / 0.1 | 1034 | 22.39 | 12 |
| A | stock | 8 | 14.96 [14.13-15.70] | 11.17 [10.77-11.97] | 12.94 [12.63-13.46] | 77 | 1296 | 31.0 / 0.2 | 1034 | 22.19 | 12 |
| A | h2-batch | 1 | 15.04 [8.94-17.70] | 11.02 [8.27-13.49] | 12.97 [9.03-13.19] | 77 | 1293 | 32.1 / 0.2 | 1034 | 22.64 | 12 |
| A | h2-batch | 2 | 14.84 [13.13-16.44] | 10.97 [10.15-12.52] | 12.75 [12.36-13.07] | 78 | 1316 | 31.2 / 0.2 | 1035 | 21.81 | 12 |
| A | h2-batch | 4 | 15.38 [14.59-17.77] | 11.36 [10.70-13.42] | 13.01 [12.88-13.25] | 77 | 1289 | 32.1 / 0.2 | 1034 | 23.00 | 12 |
| A | h2-batch | 8 | 15.79 [14.96-16.11] | 11.63 [10.82-12.10] | 13.00 [12.86-13.23] | 77 | 1291 | 32.5 / 0.2 | 1034 | 23.14 | 12 |
| Cf | stock | 1 | 15.27 [14.45-15.50] | 11.12 [10.69-11.24] | 12.88 [12.76-13.02] | 78 | 1303 | 10.4 / 0.5 | 1035 | 21.85 | 12 |
| Cf | stock | 2 | 15.38 [15.08-15.58] | 11.33 [11.19-11.52] | 12.93 [12.85-13.02] | 77 | 1297 | 32.5 / 0.4 | 1040 | 23.30 | 12 |
| Cf | stock | 4 | 15.30 [15.06-15.59] | 11.31 [11.19-11.53] | 12.93 [12.89-13.02] | 77 | 1298 | 39.6 / 0.4 | 1036 | 22.79 | 12 |
| Cf | stock | 8 | 15.30 [15.17-15.47] | 11.32 [11.11-11.37] | 12.96 [12.86-13.03] | 77 | 1295 | 39.5 / 0.4 | 1036 | 22.74 | 12 |
| Cf | h2-batch | 1 | 8.17 [7.69-8.38] | 7.35 [7.13-7.53] | 8.54 [8.27-9.13] | 117 | 1964 | 14.2 / 0.2 | 45 | 14.18 | 12 |
| Cf | h2-batch | 2 | 8.99 [8.77-9.34] | 7.81 [7.59-8.17] | 7.99 [7.80-8.21] | 125 | 2100 | 19.2 / 0.2 | 41 | 14.75 | 12 |
| Cf | h2-batch | 4 | 8.07 [7.73-9.06] | 7.49 [7.17-7.83] | 9.37 [7.90-9.75] | 107 | 1790 | 37.5 / 0.2 | 63 | 13.74 | 12 |
| Cf | h2-batch | 8 | 8.65 [8.30-9.05] | 7.67 [7.44-7.91] | 8.27 [8.14-8.59] | 121 | 2029 | 22.2 / 0.2 | 39 | 14.54 | 12 |
| Cf-cb | stock | 1 | 15.10 [9.06-15.52] | 11.17 [8.40-11.33] | 12.89 [8.90-12.99] | 78 | 1301 | 11.5 / 0.4 | 1033 | 21.79 | 12 |
| Cf-cb | stock | 2 | 15.50 [14.17-15.62] | 11.36 [10.71-11.46] | 12.94 [12.53-13.05] | 77 | 1296 | 39.8 / 0.4 | 1044 | 22.45 | 12 |
| Cf-cb | stock | 4 | 15.48 [15.34-15.74] | 11.38 [11.19-11.61] | 13.00 [12.94-13.11] | 77 | 1290 | 52.6 / 0.4 | 1040 | 22.56 | 12 |
| Cf-cb | stock | 8 | 15.47 [15.11-15.54] | 11.41 [11.14-11.50] | 12.92 [12.81-13.01] | 77 | 1298 | 51.4 / 0.3 | 1039 | 23.19 | 12 |
| Cf-cb | h2-batch | 1 | 8.85 [7.60-9.20] | 7.71 [7.09-7.99] | 7.93 [7.75-9.42] | 126 | 2115 | 16.8 / 0.1 | 44 | 14.17 | 12 |
| Cf-cb | h2-batch | 2 | 8.74 [8.27-9.24] | 7.69 [7.44-8.43] | 8.37 [8.00-8.57] | 119 | 2004 | 29.0 / 0.1 | 48 | 14.39 | 12 |
| Cf-cb | h2-batch | 4 | 9.00 [8.57-9.57] | 7.79 [7.58-8.37] | 8.02 [7.65-8.49] | 125 | 2092 | 34.6 / 0.2 | 43 | 14.58 | 12 |
| Cf-cb | h2-batch | 8 | 8.94 [8.33-9.16] | 7.81 [7.57-8.23] | 8.06 [7.73-11.27] | 124 | 2082 | 35.4 / 0.2 | 43 | 14.35 | 12 |
| Cf-m4 | stock | 1 | 18.81 [17.34-19.94] | 14.20 [13.47-14.77] | 4.39 [3.95-4.84] | 228 | 3825 | 11.6 / 0.7 | 1034 | 24.25 | 12 |
| Cf-m4 | stock | 2 | 20.82 [20.30-21.88] | 15.27 [14.96-15.92] | 4.73 [4.43-4.94] | 211 | 3546 | 36.8 / 0.7 | 1038 | 24.21 | 12 |
| Cf-m4 | stock | 4 | 21.14 [19.98-21.87] | 15.53 [14.76-15.97] | 4.82 [4.49-4.89] | 207 | 3481 | 46.6 / 0.9 | 1033 | 24.22 | 12 |
| Cf-m4 | stock | 8 | 19.97 [18.42-21.07] | 14.93 [14.24-15.54] | 4.67 [4.55-4.78] | 214 | 3594 | 41.9 / 1.0 | 1034 | 24.54 | 12 |
| Cf-m4 | h2-batch | 1 | 12.36 [11.79-13.23] | 11.15 [10.75-11.77] | 3.77 [3.51-3.96] | 265 | 4447 | 17.9 / 0.4 | 51 | 21.32 | 12 |
| Cf-m4 | h2-batch | 2 | 12.53 [12.05-13.05] | 11.31 [10.93-12.08] | 3.91 [3.54-3.98] | 256 | 4291 | 25.2 / 0.7 | 52 | 21.04 | 12 |
| Cf-m4 | h2-batch | 4 | 12.70 [12.17-13.13] | 11.31 [10.91-11.83] | 3.76 [3.52-4.04] | 266 | 4456 | 26.1 / 0.7 | 46 | 20.92 | 12 |
| Cf-m4 | h2-batch | 8 | 12.97 [12.40-13.81] | 11.55 [11.15-12.19] | 3.62 [3.40-3.95] | 276 | 4633 | 24.7 / 0.7 | 43 | 20.95 | 12 |
| Cf-cb-m4 | stock | 1 | 19.15 [17.25-20.04] | 14.08 [13.11-14.96] | 4.52 [3.91-4.78] | 221 | 3710 | 11.3 / 0.5 | 1032 | 24.20 | 12 |
| Cf-cb-m4 | stock | 2 | 20.64 [20.07-21.53] | 15.33 [14.85-16.21] | 4.71 [4.62-4.79] | 212 | 3565 | 43.0 / 1.7 | 1044 | 24.35 | 12 |
| Cf-cb-m4 | stock | 4 | 20.49 [18.18-22.15] | 15.15 [14.02-16.58] | 4.57 [4.46-4.72] | 219 | 3671 | 55.2 / 2.5 | 1037 | 24.22 | 12 |
| Cf-cb-m4 | stock | 8 | 20.87 [20.41-21.92] | 15.37 [15.06-16.51] | 4.72 [4.54-4.82] | 212 | 3554 | 56.5 / 1.7 | 1035 | 24.50 | 12 |
| Cf-cb-m4 | h2-batch | 1 | 12.82 [12.55-13.34] | 11.24 [11.03-11.74] | 3.42 [3.35-3.51] | 292 | 4900 | 16.9 / 0.4 | 46 | 21.12 | 12 |
| Cf-cb-m4 | h2-batch | 2 | 13.84 [13.37-15.30] | 12.19 [11.73-13.67] | 3.40 [3.35-3.48] | 294 | 4929 | 29.0 / 1.8 | 50 | 21.20 | 12 |
| Cf-cb-m4 | h2-batch | 4 | 13.30 [12.77-14.52] | 11.83 [11.43-13.09] | 3.55 [3.44-3.81] | 282 | 4726 | 36.8 / 1.5 | 46 | 20.87 | 12 |
| Cf-cb-m4 | h2-batch | 8 | 13.23 [12.97-14.80] | 11.76 [11.47-13.28] | 3.51 [3.44-3.86] | 285 | 4774 | 36.4 / 1.9 | 46 | 21.20 | 12 |

## c54k1

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 3.80 [3.75-3.89] | 2.57 [2.56-2.69] | 4.13 [4.08-4.23] | 242 |  | 11.1 / 0.1 | 258 | 2.84 | 24 |
| A | stock | 2 | 3.79 [3.73-3.83] | 2.57 [2.55-2.62] | 4.13 [4.08-4.18] | 242 |  | 11.1 / 0.1 | 258 | 2.86 | 24 |
| A | stock | 4 | 3.81 [3.76-3.85] | 2.58 [2.55-2.62] | 4.15 [4.11-4.19] | 241 |  | 11.1 / 0.1 | 258 | 2.86 | 24 |
| A | stock | 8 | 3.79 [3.71-3.85] | 2.57 [2.55-2.61] | 4.13 [4.04-4.19] | 242 |  | 11.1 / 0.1 | 258 | 2.86 | 24 |
| A | h2-batch | 1 | 3.77 [3.73-3.82] | 2.58 [2.55-2.60] | 4.13 [4.08-4.19] | 242 |  | 11.1 / 0.1 | 258 | 2.87 | 24 |
| A | h2-batch | 2 | 3.78 [3.65-3.93] | 2.58 [2.50-2.62] | 4.14 [4.05-4.29] | 242 |  | 11.2 / 0.1 | 258 | 2.87 | 24 |
| A | h2-batch | 4 | 3.76 [3.68-3.81] | 2.58 [2.55-2.61] | 4.12 [4.05-4.27] | 243 |  | 11.1 / 0.1 | 258 | 2.87 | 24 |
| A | h2-batch | 8 | 3.79 [3.73-3.87] | 2.57 [2.55-2.64] | 4.14 [4.08-4.20] | 242 |  | 11.2 / 0.1 | 258 | 2.86 | 24 |
| Cf | stock | 1 | 3.80 [3.74-3.83] | 2.54 [2.51-2.59] | 4.28 [4.22-4.32] | 234 |  | 5.1 / 0.1 | 260 | 2.86 | 24 |
| Cf | stock | 2 | 3.87 [3.81-3.91] | 2.63 [2.62-2.67] | 4.27 [4.22-4.32] | 234 |  | 14.8 / 0.1 | 263 | 2.86 | 24 |
| Cf | stock | 4 | 3.90 [3.88-3.94] | 2.69 [2.65-2.71] | 4.27 [4.24-4.32] | 234 |  | 15.2 / 0.1 | 261 | 2.87 | 24 |
| Cf | stock | 8 | 3.88 [3.85-4.01] | 2.68 [2.66-2.72] | 4.27 [4.25-4.40] | 234 |  | 15.1 / 0.1 | 260 | 2.86 | 24 |
| Cf | h2-batch | 1 | 1.81 [1.73-1.96] | 1.63 [1.57-1.69] | 3.06 [2.97-3.96] | 326 |  | 6.5 / 0.0 | 20 | 2.23 | 24 |
| Cf | h2-batch | 2 | 1.81 [1.79-2.11] | 1.68 [1.65-1.80] | 3.05 [2.95-3.18] | 328 |  | 13.5 / 0.0 | 22 | 2.22 | 24 |
| Cf | h2-batch | 4 | 1.86 [1.82-2.09] | 1.72 [1.69-1.78] | 3.55 [3.07-7.41] | 282 |  | 12.2 / 0.0 | 21 | 2.76 | 24 |
| Cf | h2-batch | 8 | 1.93 [1.84-2.01] | 1.72 [1.69-1.79] | 3.16 [3.02-3.75] | 317 |  | 10.4 / 0.0 | 20 | 2.36 | 24 |
| Cf-cb | stock | 1 | 3.80 [3.76-3.86] | 2.55 [2.52-2.59] | 4.27 [4.23-4.33] | 234 |  | 4.2 / 0.1 | 259 | 2.85 | 24 |
| Cf-cb | stock | 2 | 3.87 [3.79-3.93] | 2.64 [2.62-2.67] | 4.28 [4.19-4.31] | 234 |  | 13.2 / 0.1 | 262 | 2.87 | 24 |
| Cf-cb | stock | 4 | 3.87 [3.83-3.95] | 2.67 [2.63-2.69] | 4.29 [4.22-4.35] | 233 |  | 12.6 / 0.1 | 259 | 2.86 | 24 |
| Cf-cb | stock | 8 | 3.89 [3.84-4.02] | 2.68 [2.64-2.72] | 4.26 [4.18-4.42] | 235 |  | 13.6 / 0.1 | 260 | 2.86 | 24 |
| Cf-cb | h2-batch | 1 | 1.78 [1.73-2.03] | 1.61 [1.58-1.73] | 3.41 [3.02-5.65] | 293 |  | 6.1 / 0.0 | 19 | 2.66 | 24 |
| Cf-cb | h2-batch | 2 | 1.89 [1.81-1.96] | 1.69 [1.65-1.72] | 3.14 [3.03-5.38] | 319 |  | 7.7 / 0.0 | 20 | 2.31 | 24 |
| Cf-cb | h2-batch | 4 | 1.79 [1.73-1.84] | 1.68 [1.66-1.72] | 3.29 [3.25-7.94] | 304 |  | 12.9 / 0.0 | 20 | 2.35 | 24 |
| Cf-cb | h2-batch | 8 | 2.05 [1.79-2.14] | 1.76 [1.69-1.83] | 3.09 [2.98-7.74] | 324 |  | 9.4 / 0.0 | 20 | 2.31 | 24 |

## c54k8

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 3.52 [3.40-3.84] | 2.66 [2.57-3.03] | 3.57 [3.49-3.66] | 280 |  | 8.8 / 0.0 | 259 | 6.96 | 18 |
| A | stock | 2 | 3.58 [3.06-3.83] | 2.68 [2.59-2.95] | 3.50 [3.23-3.57] | 286 |  | 9.1 / 0.0 | 259 | 6.70 | 18 |
| A | stock | 4 | 3.59 [2.89-4.05] | 2.63 [2.49-3.04] | 3.46 [2.88-3.59] | 289 |  | 9.4 / 0.0 | 259 | 6.53 | 18 |
| A | stock | 8 | 3.79 [3.34-4.05] | 2.84 [2.51-3.16] | 3.56 [3.49-3.70] | 281 |  | 9.5 / 0.0 | 258 | 6.65 | 18 |
| A | h2-batch | 1 | 3.71 [3.36-4.15] | 2.77 [2.55-3.14] | 3.56 [3.39-3.68] | 281 |  | 9.6 / 0.0 | 259 | 6.74 | 18 |
| A | h2-batch | 2 | 2.77 [2.42-3.43] | 2.36 [2.10-2.75] | 2.98 [2.88-3.38] | 336 |  | 9.2 / 0.0 | 259 | 5.38 | 18 |
| A | h2-batch | 4 | 3.73 [3.33-4.05] | 2.79 [2.61-3.15] | 3.53 [3.35-3.75] | 283 |  | 9.3 / 0.0 | 259 | 6.90 | 18 |
| A | h2-batch | 8 | 3.63 [3.46-3.96] | 2.71 [2.58-3.01] | 3.55 [3.46-3.67] | 282 |  | 9.1 / 0.0 | 259 | 6.88 | 18 |
| Cf | stock | 1 | 3.91 [3.44-3.97] | 2.98 [2.81-3.04] | 3.60 [3.42-3.70] | 278 |  | 5.4 / 0.1 | 258 | 7.53 | 18 |
| Cf | stock | 2 | 4.01 [3.93-4.08] | 3.07 [3.01-3.12] | 3.61 [3.51-3.74] | 277 |  | 10.5 / 0.1 | 260 | 7.44 | 18 |
| Cf | stock | 4 | 4.01 [2.75-4.06] | 3.06 [2.49-3.10] | 3.55 [2.90-3.67] | 281 |  | 12.9 / 0.0 | 259 | 6.89 | 18 |
| Cf | stock | 8 | 4.02 [3.90-4.13] | 3.07 [3.00-3.16] | 3.61 [3.55-3.69] | 277 |  | 13.2 / 0.1 | 258 | 7.22 | 18 |
| Cf | h2-batch | 1 | 2.21 [2.13-2.38] | 2.06 [1.99-2.09] | 3.05 [2.66-3.16] | 328 |  | 9.5 / 0.0 | 18 | 5.40 | 18 |
| Cf | h2-batch | 2 | 2.28 [2.20-2.38] | 2.07 [2.03-2.12] | 2.90 [2.70-3.57] | 345 |  | 8.5 / 0.0 | 12 | 5.30 | 18 |
| Cf | h2-batch | 4 | 2.24 [2.18-2.31] | 2.06 [2.01-2.13] | 2.98 [2.76-3.14] | 336 |  | 9.6 / 0.0 | 12 | 5.61 | 18 |
| Cf | h2-batch | 8 | 2.27 [2.18-2.42] | 2.10 [2.00-2.14] | 2.79 [2.61-3.02] | 358 |  | 10.2 / 0.0 | 12 | 5.32 | 18 |
| Cf-cb | stock | 1 | 3.40 [2.32-3.60] | 2.49 [2.01-2.57] | 3.55 [2.93-3.67] | 281 |  | 5.0 / 0.1 | 257 | 6.67 | 18 |
| Cf-cb | stock | 2 | 3.59 [2.79-3.61] | 2.64 [2.27-2.65] | 3.54 [3.21-3.58] | 283 |  | 11.0 / 0.1 | 260 | 6.95 | 18 |
| Cf-cb | stock | 4 | 3.69 [3.55-3.76] | 2.72 [2.67-2.77] | 3.59 [3.52-3.67] | 279 |  | 14.5 / 0.0 | 259 | 6.85 | 18 |
| Cf-cb | stock | 8 | 3.46 [3.01-3.74] | 2.61 [2.41-2.73] | 3.64 [3.40-3.69] | 275 |  | 13.4 / 0.0 | 259 | 6.92 | 18 |
| Cf-cb | h2-batch | 1 | 1.71 [1.68-1.94] | 1.59 [1.56-1.65] | 2.71 [2.49-2.97] | 370 |  | 9.4 / 0.0 | 18 | 4.79 | 18 |
| Cf-cb | h2-batch | 2 | 1.96 [1.78-1.98] | 1.69 [1.61-1.74] | 2.69 [2.49-2.93] | 372 |  | 8.7 / 0.0 | 14 | 5.22 | 18 |
| Cf-cb | h2-batch | 4 | 1.88 [1.84-1.96] | 1.72 [1.68-1.76] | 2.82 [2.70-3.21] | 354 |  | 11.6 / 0.0 | 15 | 5.18 | 18 |
| Cf-cb | h2-batch | 8 | 1.90 [1.79-2.00] | 1.71 [1.64-1.76] | 2.78 [2.35-3.25] | 360 |  | 11.6 / 0.0 | 15 | 4.94 | 18 |

## c54k16

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 3.70 [3.41-4.19] | 2.89 [2.58-3.34] | 3.41 [2.85-3.52] | 294 |  | 8.8 / 0.1 | 259 | 7.14 | 18 |
| A | stock | 2 | 3.77 [3.39-4.42] | 2.89 [2.51-3.48] | 3.44 [3.36-3.57] | 290 |  | 8.6 / 0.1 | 258 | 7.18 | 18 |
| A | stock | 4 | 3.86 [3.40-4.25] | 3.02 [2.59-3.36] | 3.42 [3.31-3.50] | 293 |  | 8.5 / 0.0 | 259 | 6.51 | 18 |
| A | stock | 8 | 3.73 [3.49-4.97] | 2.91 [2.61-3.82] | 3.44 [3.32-3.66] | 291 |  | 9.0 / 0.1 | 258 | 7.10 | 18 |
| A | h2-batch | 1 | 3.95 [3.55-4.50] | 3.09 [2.60-3.56] | 3.40 [3.36-3.53] | 294 |  | 9.0 / 0.1 | 259 | 6.89 | 18 |
| A | h2-batch | 2 | 3.58 [2.58-4.09] | 2.79 [2.35-3.21] | 3.32 [2.71-3.41] | 301 |  | 8.9 / 0.0 | 259 | 6.29 | 18 |
| A | h2-batch | 4 | 3.55 [3.20-4.31] | 2.85 [2.58-3.42] | 3.40 [3.11-3.55] | 294 |  | 8.3 / 0.0 | 259 | 7.28 | 18 |
| A | h2-batch | 8 | 3.54 [2.38-4.13] | 2.82 [2.23-3.29] | 3.38 [2.96-3.45] | 296 |  | 9.2 / 0.0 | 259 | 6.96 | 18 |
| Cf | stock | 1 | 3.94 [3.71-4.86] | 2.99 [2.88-4.00] | 3.48 [3.42-3.55] | 287 |  | 4.7 / 0.1 | 257 | 7.46 | 18 |
| Cf | stock | 2 | 3.95 [3.74-5.02] | 3.02 [2.92-4.14] | 3.46 [3.41-3.60] | 289 |  | 10.1 / 0.1 | 259 | 7.68 | 18 |
| Cf | stock | 4 | 3.45 [2.52-3.95] | 2.79 [2.33-3.46] | 3.31 [2.70-3.55] | 302 |  | 11.6 / 0.1 | 259 | 6.84 | 18 |
| Cf | stock | 8 | 3.84 [2.55-3.97] | 2.96 [2.40-3.55] | 3.43 [2.88-3.53] | 291 |  | 11.9 / 0.1 | 258 | 7.55 | 18 |
| Cf | h2-batch | 1 | 2.35 [2.24-3.30] | 2.07 [2.01-3.12] | 2.56 [2.41-2.91] | 390 |  | 6.2 / 0.0 | 11 | 5.24 | 18 |
| Cf | h2-batch | 2 | 2.28 [2.16-3.23] | 2.04 [1.98-3.09] | 2.64 [2.44-2.94] | 378 |  | 8.8 / 0.0 | 13 | 5.07 | 18 |
| Cf | h2-batch | 4 | 2.22 [2.10-3.31] | 2.04 [1.97-3.18] | 2.79 [2.34-3.06] | 358 |  | 12.1 / 0.0 | 18 | 5.24 | 18 |
| Cf | h2-batch | 8 | 2.32 [2.20-3.28] | 2.03 [2.01-3.10] | 2.57 [2.33-2.72] | 389 |  | 9.1 / 0.0 | 11 | 5.23 | 18 |
| Cf-cb | stock | 1 | 3.52 [3.47-4.40] | 2.54 [2.52-3.40] | 3.51 [3.42-3.65] | 285 |  | 4.4 / 0.1 | 257 | 7.24 | 18 |
| Cf-cb | stock | 2 | 3.54 [2.98-4.14] | 2.61 [2.35-3.39] | 3.41 [3.24-3.57] | 293 |  | 10.6 / 0.1 | 260 | 7.15 | 18 |
| Cf-cb | stock | 4 | 3.22 [2.15-4.36] | 2.48 [1.99-3.65] | 3.35 [2.74-3.50] | 298 |  | 13.2 / 0.0 | 259 | 6.79 | 18 |
| Cf-cb | stock | 8 | 3.18 [2.29-3.49] | 2.48 [2.06-3.23] | 3.27 [2.78-3.49] | 306 |  | 13.2 / 0.0 | 259 | 6.94 | 18 |
| Cf-cb | h2-batch | 1 | 1.71 [1.68-2.69] | 1.57 [1.51-2.56] | 2.58 [2.43-2.91] | 388 |  | 5.4 / 0.0 | 13 | 5.05 | 18 |
| Cf-cb | h2-batch | 2 | 1.93 [1.77-2.77] | 1.64 [1.58-2.58] | 2.45 [2.31-2.68] | 408 |  | 8.3 / 0.0 | 13 | 5.13 | 18 |
| Cf-cb | h2-batch | 4 | 1.93 [1.81-2.74] | 1.67 [1.66-2.61] | 2.58 [2.32-2.73] | 388 |  | 10.9 / 0.0 | 13 | 5.24 | 18 |
| Cf-cb | h2-batch | 8 | 1.89 [1.83-2.77] | 1.68 [1.63-2.57] | 2.49 [2.34-2.71] | 401 |  | 11.0 / 0.0 | 12 | 4.92 | 18 |
| Cf-m4 | stock | 1 | 4.78 [4.44-5.92] | 3.68 [3.48-4.79] | 1.43 [1.34-1.55] | 700 |  | 5.7 / 0.2 | 258 | 6.84 | 18 |
| Cf-m4 | stock | 2 | 5.08 [4.64-5.64] | 3.89 [3.64-4.64] | 1.45 [1.34-1.59] | 688 |  | 12.0 / 0.2 | 260 | 7.22 | 18 |
| Cf-m4 | stock | 4 | 4.89 [4.37-5.75] | 3.79 [3.54-4.79] | 1.47 [1.38-1.50] | 683 |  | 13.3 / 0.2 | 259 | 7.40 | 18 |
| Cf-m4 | stock | 8 | 5.10 [4.47-5.83] | 3.92 [3.59-4.84] | 1.46 [1.43-1.52] | 683 |  | 13.3 / 0.2 | 259 | 7.33 | 18 |
| Cf-m4 | h2-batch | 1 | 2.92 [2.84-3.81] | 2.66 [2.57-3.58] | 1.35 [1.25-1.47] | 741 |  | 6.9 / 0.1 | 15 | 6.88 | 18 |
| Cf-m4 | h2-batch | 2 | 2.96 [2.79-3.97] | 2.68 [2.55-3.80] | 1.35 [1.23-1.53] | 741 |  | 10.1 / 0.1 | 16 | 6.46 | 18 |
| Cf-m4 | h2-batch | 4 | 2.86 [2.80-3.96] | 2.65 [2.59-3.81] | 1.36 [1.24-1.52] | 738 |  | 12.2 / 0.2 | 17 | 6.96 | 18 |
| Cf-m4 | h2-batch | 8 | 2.93 [2.85-3.92] | 2.65 [2.59-3.70] | 1.39 [1.30-1.57] | 718 |  | 10.6 / 0.2 | 15 | 6.88 | 18 |
| Cf-cb-m4 | stock | 1 | 4.10 [3.85-4.74] | 3.09 [2.98-3.78] | 1.36 [1.30-1.82] | 734 |  | 5.5 / 0.1 | 257 | 6.09 | 18 |
| Cf-cb-m4 | stock | 2 | 4.47 [3.99-4.99] | 3.34 [3.12-3.94] | 1.40 [1.32-1.97] | 714 |  | 12.8 / 0.0 | 260 | 6.57 | 18 |
| Cf-cb-m4 | stock | 4 | 4.54 [4.20-4.98] | 3.39 [3.23-3.94] | 1.40 [1.31-2.03] | 714 |  | 15.4 / 0.0 | 260 | 6.42 | 18 |
| Cf-cb-m4 | stock | 8 | 4.39 [4.05-5.04] | 3.35 [3.15-4.05] | 1.40 [1.33-1.99] | 713 |  | 15.2 / 0.1 | 260 | 6.98 | 18 |
| Cf-cb-m4 | h2-batch | 1 | 2.48 [2.34-3.05] | 2.21 [2.08-2.78] | 1.31 [1.27-1.93] | 765 |  | 6.5 / 0.0 | 19 | 5.81 | 18 |
| Cf-cb-m4 | h2-batch | 2 | 2.68 [2.57-3.21] | 2.41 [2.33-2.97] | 1.33 [1.18-1.90] | 750 |  | 11.2 / 0.0 | 21 | 5.49 | 18 |
| Cf-cb-m4 | h2-batch | 4 | 2.60 [2.51-3.27] | 2.35 [2.28-3.08] | 1.37 [1.20-1.83] | 728 |  | 12.4 / 0.0 | 20 | 6.30 | 18 |
| Cf-cb-m4 | h2-batch | 8 | 2.73 [2.60-3.33] | 2.47 [2.32-3.09] | 1.35 [1.17-1.82] | 742 |  | 12.6 / 0.0 | 20 | 6.00 | 18 |

## c54k32

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 4.12 [3.52-4.51] | 3.20 [2.98-3.59] | 3.39 [3.27-3.52] | 295 |  | 8.9 / 0.0 | 259 | 7.30 | 15 |
| A | stock | 2 | 4.38 [3.95-4.78] | 3.44 [3.04-3.77] | 3.48 [3.39-3.68] | 288 |  | 8.8 / 0.1 | 259 | 7.59 | 15 |
| A | stock | 4 | 4.06 [3.62-4.73] | 3.23 [2.96-3.87] | 3.41 [3.32-3.63] | 293 |  | 8.8 / 0.0 | 259 | 7.27 | 15 |
| A | stock | 8 | 4.03 [2.83-4.63] | 3.13 [2.66-3.68] | 3.43 [2.71-3.57] | 292 |  | 8.8 / 0.0 | 259 | 7.39 | 15 |
| A | h2-batch | 1 | 4.05 [3.55-4.40] | 3.18 [2.71-3.55] | 3.40 [3.04-3.48] | 294 |  | 8.6 / 0.0 | 258 | 7.43 | 15 |
| A | h2-batch | 2 | 4.25 [3.80-4.55] | 3.39 [2.87-3.54] | 3.43 [3.35-3.66] | 291 |  | 8.7 / 0.0 | 258 | 7.35 | 15 |
| A | h2-batch | 4 | 3.99 [3.56-4.66] | 3.11 [2.87-3.75] | 3.42 [3.35-3.48] | 293 |  | 8.3 / 0.0 | 258 | 7.30 | 15 |
| A | h2-batch | 8 | 4.20 [3.25-4.50] | 3.32 [2.84-3.55] | 3.43 [2.92-3.64] | 292 |  | 8.8 / 0.0 | 259 | 7.52 | 15 |
| Cf | stock | 1 | 3.74 [3.57-4.98] | 2.85 [2.77-4.06] | 3.41 [3.35-3.46] | 294 |  | 4.7 / 0.1 | 258 | 7.89 | 15 |
| Cf | stock | 2 | 3.81 [2.47-4.95] | 2.93 [2.28-4.07] | 3.39 [2.72-3.53] | 295 |  | 9.8 / 0.1 | 259 | 7.89 | 15 |
| Cf | stock | 4 | 3.51 [2.46-4.55] | 2.78 [2.27-3.86] | 3.31 [2.61-3.54] | 302 |  | 11.3 / 0.1 | 258 | 7.72 | 15 |
| Cf | stock | 8 | 3.90 [3.29-5.02] | 2.96 [2.68-4.08] | 3.42 [3.28-3.60] | 292 |  | 11.4 / 0.1 | 258 | 8.13 | 15 |
| Cf | h2-batch | 1 | 2.28 [2.20-3.34] | 2.01 [1.95-3.14] | 2.56 [2.32-2.77] | 391 |  | 5.9 / 0.0 | 11 | 5.59 | 15 |
| Cf | h2-batch | 2 | 2.29 [2.17-3.39] | 2.01 [1.95-3.15] | 2.48 [2.33-2.70] | 403 |  | 7.1 / 0.0 | 10 | 5.59 | 15 |
| Cf | h2-batch | 4 | 2.27 [2.12-3.32] | 2.00 [1.92-3.10] | 2.62 [2.37-2.75] | 382 |  | 7.9 / 0.1 | 10 | 5.74 | 15 |
| Cf | h2-batch | 8 | 2.18 [2.10-3.40] | 1.97 [1.92-3.21] | 2.55 [2.44-2.78] | 391 |  | 8.0 / 0.1 | 10 | 5.53 | 15 |
| Cf-cb | stock | 1 | 3.23 [2.03-4.27] | 2.42 [1.85-3.38] | 3.41 [2.59-3.44] | 293 |  | 4.3 / 0.1 | 258 | 7.43 | 15 |
| Cf-cb | stock | 2 | 3.52 [2.97-4.91] | 2.59 [2.33-3.86] | 3.38 [3.20-3.69] | 296 |  | 9.9 / 0.1 | 260 | 7.73 | 15 |
| Cf-cb | stock | 4 | 3.52 [2.86-3.91] | 2.60 [2.32-3.40] | 3.24 [2.90-3.54] | 308 |  | 13.2 / 0.1 | 259 | 7.43 | 15 |
| Cf-cb | stock | 8 | 3.59 [3.49-4.55] | 2.67 [2.62-3.59] | 3.40 [3.34-3.50] | 294 |  | 13.7 / 0.0 | 259 | 7.73 | 15 |
| Cf-cb | h2-batch | 1 | 1.85 [1.64-2.77] | 1.57 [1.52-2.57] | 2.38 [2.33-2.80] | 420 |  | 5.1 / 0.0 | 12 | 5.25 | 15 |
| Cf-cb | h2-batch | 2 | 1.89 [1.73-2.81] | 1.61 [1.55-2.61] | 2.46 [2.27-2.68] | 406 |  | 8.1 / 0.0 | 13 | 5.39 | 15 |
| Cf-cb | h2-batch | 4 | 1.95 [1.82-3.04] | 1.68 [1.64-2.84] | 2.43 [2.32-2.63] | 411 |  | 11.3 / 0.0 | 12 | 5.34 | 15 |
| Cf-cb | h2-batch | 8 | 2.00 [1.73-2.97] | 1.71 [1.58-2.67] | 2.42 [2.29-2.72] | 413 |  | 11.0 / 0.0 | 12 | 5.29 | 15 |
| Cf-m4 | stock | 1 | 4.78 [3.94-5.54] | 3.67 [3.27-4.58] | 1.35 [1.31-1.51] | 739 |  | 5.5 / 0.1 | 258 | 7.62 | 15 |
| Cf-m4 | stock | 2 | 5.01 [4.10-5.74] | 3.88 [3.39-4.76] | 1.37 [1.31-1.49] | 729 |  | 10.8 / 0.1 | 259 | 7.80 | 15 |
| Cf-m4 | stock | 4 | 4.91 [4.46-5.92] | 3.80 [3.58-4.87] | 1.43 [1.31-1.56] | 698 |  | 12.5 / 0.2 | 258 | 7.81 | 15 |
| Cf-m4 | stock | 8 | 4.87 [4.32-5.75] | 3.79 [3.54-4.81] | 1.39 [1.34-1.48] | 719 |  | 12.6 / 0.2 | 258 | 7.65 | 15 |
| Cf-m4 | h2-batch | 1 | 2.86 [2.72-3.87] | 2.62 [2.48-3.64] | 1.28 [1.20-1.38] | 779 |  | 6.4 / 0.1 | 13 | 7.14 | 15 |
| Cf-m4 | h2-batch | 2 | 2.84 [2.73-3.92] | 2.61 [2.52-3.73] | 1.34 [1.18-1.43] | 747 |  | 10.1 / 0.1 | 15 | 7.10 | 15 |
| Cf-m4 | h2-batch | 4 | 2.84 [2.76-3.91] | 2.60 [2.53-3.71] | 1.36 [1.20-1.49] | 735 |  | 9.8 / 0.1 | 14 | 6.86 | 15 |
| Cf-m4 | h2-batch | 8 | 2.84 [2.77-3.96] | 2.59 [2.52-3.74] | 1.30 [1.27-1.38] | 772 |  | 9.8 / 0.2 | 14 | 7.14 | 15 |
| Cf-cb-m4 | stock | 1 | 4.00 [3.55-4.66] | 3.05 [2.90-3.76] | 1.27 [1.23-1.88] | 785 |  | 5.4 / 0.1 | 258 | 7.08 | 15 |
| Cf-cb-m4 | stock | 2 | 4.53 [4.20-5.18] | 3.41 [3.23-4.13] | 1.33 [1.28-1.96] | 750 |  | 11.6 / 0.0 | 260 | 7.48 | 15 |
| Cf-cb-m4 | stock | 4 | 4.65 [4.25-5.10] | 3.49 [3.30-4.05] | 1.33 [1.24-1.98] | 751 |  | 15.3 / 0.0 | 260 | 7.18 | 15 |
| Cf-cb-m4 | stock | 8 | 4.55 [4.22-5.27] | 3.44 [3.33-4.22] | 1.30 [1.27-2.04] | 772 |  | 15.3 / 0.1 | 259 | 7.20 | 15 |
| Cf-cb-m4 | h2-batch | 1 | 2.51 [2.32-3.24] | 2.24 [2.08-2.98] | 1.22 [1.07-1.84] | 820 |  | 5.8 / 0.0 | 16 | 6.49 | 15 |
| Cf-cb-m4 | h2-batch | 2 | 2.66 [2.49-3.35] | 2.44 [2.26-3.14] | 1.21 [1.12-1.81] | 828 |  | 11.0 / 0.0 | 19 | 6.65 | 15 |
| Cf-cb-m4 | h2-batch | 4 | 2.70 [2.54-3.28] | 2.42 [2.28-3.09] | 1.22 [1.17-1.85] | 818 |  | 12.0 / 0.0 | 17 | 6.59 | 15 |
| Cf-cb-m4 | h2-batch | 8 | 2.81 [2.51-3.39] | 2.52 [2.28-3.13] | 1.19 [1.12-1.82] | 843 |  | 12.1 / 0.0 | 17 | 6.44 | 15 |
