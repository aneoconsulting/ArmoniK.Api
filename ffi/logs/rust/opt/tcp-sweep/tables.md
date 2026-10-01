# TCP core worker sweep (gen/tcp_sweep.sh)

## header (low)

```
# in-process comparison: commit 1b3388b2; 2026-10-01T19:23:06Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x10, 0,9-10,19 x39, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 1.54 1.99 1.81 2/931 899978
# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts
# transport tcp (client target http://127.0.0.1:39973, TCP_NODELAY read back on every client socket); base env AK_PROBE_TASKCLOCK=1 AK_PROBE_SERVER_PID=899581 AK_RPC_TARGET=http://127.0.0.1:39973 AK_EXPECT_NODELAY=1
# server also on TCP 127.0.0.1:39973 (pinned configuration, TCP_NODELAY on accept)
# netfilter (read-only): modules nft_log nft_limit xt_limit xt_NFLOG nfnetlink_log xt_physdev xt_multiport nf_conntrack_netlink xt_mark xt_nfacct nfnetlink_acct xt_comment xt_set ip_set xt_addrtype xt_CHECKSUM xt_MASQUERADE xt_conntrack ipt_REJECT nf_reject_ipv4 xt_tcpudp nft_compat nft_chain_nat nf_tables nfnetlink ip6_udp_tunnel nfit xt_nat br_netfilter nf_nat bridge nf_conntrack nf_defrag_ipv6 nf_defrag_ipv4 
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 899581, pinned configuration, transport tcp
# cells A,Cf,Cf-cb (direction c: A,Cf,Cf-cb) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 c54k1 c54k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2; d16k16 5 x 1 batch of 16 warm 1; d16k32 4 x 1 x 32 warm 1; c54k16 6 x 2 x 16 warm 1; c54k32 5 x 2 x 32 warm 1); repetitions 3
# condition stock-w1: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=1,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w1: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=1,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w2: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=2,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w2: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=2,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w4: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=4,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w4: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=4,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition stock-w8: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch-w8: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,AK_CORE_WORKERS=8,AK_HOST_WORKERS=8,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 310 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x10, 0,9-10,19 x39, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 1.90 1.99 1.87 1/942 908543
```

## header (high)

```
# in-process comparison: commit 1b3388b2 + UNCOMMITTED; 2026-10-01T19:32:42Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x10, 0,9-10,19 x39, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.99 2.54 2.13 1/940 915175
# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts
# transport tcp (client target http://127.0.0.1:33325, TCP_NODELAY read back on every client socket); base env AK_PROBE_TASKCLOCK=1 AK_PROBE_SERVER_PID=914807 AK_RPC_TARGET=http://127.0.0.1:33325 AK_EXPECT_NODELAY=1
# server also on TCP 127.0.0.1:33325 (pinned configuration, TCP_NODELAY on accept)
# netfilter (read-only): modules nft_log nft_limit xt_limit xt_NFLOG nfnetlink_log xt_physdev xt_multiport nf_conntrack_netlink xt_mark xt_nfacct nfnetlink_acct xt_comment xt_set ip_set xt_addrtype xt_CHECKSUM xt_MASQUERADE xt_conntrack ipt_REJECT nf_reject_ipv4 xt_tcpudp nft_compat nft_chain_nat nf_tables nfnetlink ip6_udp_tunnel nfit xt_nat br_netfilter nf_nat bridge nf_conntrack nf_defrag_ipv6 nf_defrag_ipv4 
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 914807, pinned configuration, transport tcp
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
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x10, 0,9-10,19 x39, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 2.90 3.32 2.79 1/943 934122
```

## Notes

- MACHINE CONDITION: the machine was suspended a second time, from 2026-10-01 19:53:03 to 20:36:33 local
  (17:53:03Z to 18:36:33Z; kernel "PM: suspend entry/exit"), although a sleep inhibitor had been
  reported in place. After that resume the IRQ affinities were NOT re-pinned before this sweep (low session
  started 19:23:06Z, high 19:32:42Z): the headers show 10 IRQs on 0-19 and 39 on 0,9-10,19 (the baseline
  after the first re-pin was 2 on 0-19 and 47 on 0,9-10,19). Effective CPUs of the 0-19 IRQs, read after the
  sweep: mei_me 1, enp5s0 4, enp5s0-rx-0 5, rx-1 6, tx-0 7, tx-1 8, nvme0q0 2, eno1 3 (all benchmark
  CPUs). Rates read after the sweep over 20 s: eno1 150 interrupts on CPU 3 (about 7.5/s), every other one 0.
  The loopback path raises no device interrupt. The gate (p1-landed) and the h2-variant checks and smoke
  ran in the same state (correctness only).
- Affinity line present in all 192 jsonl; TCP_NODELAY on every live client socket of every process
  (3 or 11 sockets); the server's thread count was 9 at the start and end of every round.
- Benchmark wall: low 310 s, high 578 s (888 s in all).

CPU = client task-clock; P5.4 throughput in calls/s only.


## d16k1

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 14.86 [14.72-15.35] | 10.04 [9.91-10.46] | 14.51 [14.11-14.61] | 69 | 1156 | 37.0 / 0.4 | 1035 | 11.08 | 24 |
| A | stock | 2 | 14.97 [14.84-15.42] | 10.07 [9.98-10.23] | 14.63 [14.50-14.82] | 68 | 1147 | 37.0 / 0.3 | 1035 | 11.09 | 24 |
| A | stock | 4 | 14.96 [14.78-15.26] | 10.02 [9.92-10.16] | 14.66 [14.47-14.87] | 68 | 1145 | 36.7 / 0.4 | 1035 | 11.09 | 24 |
| A | stock | 8 | 15.00 [14.90-15.16] | 10.04 [9.99-10.09] | 14.69 [14.52-14.83] | 68 | 1142 | 37.0 / 0.4 | 1035 | 11.12 | 24 |
| A | h2-batch | 1 | 15.02 [14.89-15.16] | 9.98 [9.90-10.11] | 14.69 [14.59-14.89] | 68 | 1142 | 35.6 / 0.4 | 1035 | 11.09 | 24 |
| A | h2-batch | 2 | 15.07 [14.92-15.43] | 10.09 [9.94-10.34] | 14.69 [14.54-14.88] | 68 | 1142 | 36.6 / 0.4 | 1035 | 11.12 | 24 |
| A | h2-batch | 4 | 15.04 [14.82-15.34] | 10.07 [9.93-10.24] | 14.71 [14.48-14.98] | 68 | 1140 | 36.6 / 0.4 | 1035 | 11.15 | 24 |
| A | h2-batch | 8 | 15.00 [14.83-15.14] | 10.03 [9.96-10.09] | 14.70 [14.53-14.94] | 68 | 1141 | 35.3 / 0.4 | 1035 | 11.11 | 24 |
| Cf | stock | 1 | 14.73 [14.57-14.87] | 10.07 [9.98-10.15] | 13.04 [12.90-13.22] | 77 | 1287 | 10.4 / 0.4 | 1028 | 10.60 | 24 |
| Cf | stock | 2 | 15.11 [13.71-15.48] | 10.48 [9.92-10.64] | 13.14 [12.48-13.45] | 76 | 1277 | 40.2 / 0.4 | 1042 | 10.66 | 24 |
| Cf | stock | 4 | 15.35 [15.23-15.48] | 10.60 [10.45-10.68] | 13.43 [13.30-13.53] | 74 | 1249 | 45.6 / 0.2 | 1036 | 10.71 | 24 |
| Cf | stock | 8 | 15.31 [15.09-15.60] | 10.58 [10.49-10.77] | 13.33 [13.14-13.57] | 75 | 1258 | 45.8 / 0.2 | 1037 | 10.70 | 24 |
| Cf | h2-batch | 1 | 7.32 [7.00-7.67] | 6.53 [6.35-6.67] | 8.90 [8.74-10.44] | 112 | 1886 | 14.4 / 0.1 | 76 | 9.09 | 24 |
| Cf | h2-batch | 2 | 8.13 [7.91-8.27] | 6.96 [6.85-7.29] | 8.47 [8.22-9.99] | 118 | 1981 | 18.8 / 0.0 | 77 | 8.35 | 24 |
| Cf | h2-batch | 4 | 7.65 [7.46-8.04] | 6.73 [6.69-6.93] | 8.88 [8.73-9.66] | 113 | 1888 | 23.8 / 0.0 | 78 | 8.89 | 24 |
| Cf | h2-batch | 8 | 7.86 [7.53-8.15] | 6.86 [6.67-6.96] | 9.03 [8.64-11.33] | 111 | 1859 | 22.1 / 0.0 | 77 | 8.76 | 24 |
| Cf-cb | stock | 1 | 14.73 [14.55-14.93] | 10.18 [10.10-10.32] | 12.89 [12.75-13.09] | 78 | 1302 | 21.4 / 0.4 | 1028 | 10.54 | 24 |
| Cf-cb | stock | 2 | 15.47 [15.36-15.75] | 10.76 [10.72-10.85] | 13.32 [13.22-13.53] | 75 | 1259 | 62.7 / 0.3 | 1054 | 10.74 | 24 |
| Cf-cb | stock | 4 | 15.79 [13.77-15.99] | 11.03 [10.40-11.12] | 13.51 [12.03-13.66] | 74 | 1242 | 75.8 / 0.3 | 1049 | 10.76 | 24 |
| Cf-cb | stock | 8 | 15.81 [15.58-15.88] | 11.00 [10.95-11.10] | 13.49 [13.30-13.55] | 74 | 1244 | 77.9 / 0.2 | 1049 | 10.75 | 24 |
| Cf-cb | h2-batch | 1 | 7.49 [6.94-8.20] | 6.70 [6.42-7.03] | 9.15 [8.59-10.16] | 109 | 1835 | 28.0 / 0.0 | 78 | 9.10 | 24 |
| Cf-cb | h2-batch | 2 | 7.79 [7.41-8.32] | 7.00 [6.92-7.41] | 9.76 [8.97-10.22] | 102 | 1718 | 48.6 / 0.0 | 90 | 9.02 | 24 |
| Cf-cb | h2-batch | 4 | 8.74 [8.39-8.84] | 7.56 [7.37-7.64] | 8.55 [8.33-9.57] | 117 | 1963 | 56.9 / 0.0 | 86 | 8.51 | 24 |
| Cf-cb | h2-batch | 8 | 7.64 [7.52-8.73] | 7.09 [6.98-7.54] | 9.54 [9.29-9.90] | 105 | 1759 | 67.1 / 0.0 | 85 | 8.61 | 24 |

## d16k8

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 14.04 [10.11-15.05] | 10.47 [9.14-11.18] | 12.49 [9.38-12.91] | 80 | 1344 | 30.9 / 0.1 | 1035 | 20.13 | 18 |
| A | stock | 2 | 14.81 [10.05-15.47] | 10.88 [8.71-11.52] | 12.87 [9.40-13.04] | 78 | 1304 | 31.3 / 0.1 | 1034 | 21.59 | 18 |
| A | stock | 4 | 14.84 [14.50-16.42] | 10.96 [10.69-12.39] | 13.10 [12.89-13.39] | 76 | 1280 | 32.8 / 0.2 | 1033 | 21.44 | 18 |
| A | stock | 8 | 15.07 [9.71-16.25] | 11.03 [8.91-12.10] | 12.96 [9.54-13.22] | 77 | 1294 | 34.0 / 0.1 | 1033 | 21.84 | 18 |
| A | h2-batch | 1 | 15.23 [9.32-16.28] | 11.02 [8.48-11.90] | 12.93 [8.99-13.32] | 77 | 1298 | 33.4 / 0.2 | 1033 | 21.22 | 18 |
| A | h2-batch | 2 | 15.36 [13.88-15.96] | 11.17 [10.37-11.73] | 13.14 [12.66-13.35] | 76 | 1276 | 33.7 / 0.1 | 1033 | 21.88 | 18 |
| A | h2-batch | 4 | 15.10 [13.57-15.64] | 11.12 [10.34-11.48] | 12.89 [12.43-13.24] | 78 | 1301 | 33.2 / 0.1 | 1033 | 21.84 | 18 |
| A | h2-batch | 8 | 14.95 [14.12-15.38] | 10.97 [10.62-11.59] | 12.87 [12.68-13.30] | 78 | 1304 | 32.0 / 0.1 | 1034 | 21.03 | 18 |
| Cf | stock | 1 | 15.31 [9.60-15.62] | 11.14 [8.69-11.36] | 13.06 [9.25-13.19] | 77 | 1284 | 10.7 / 0.4 | 1034 | 20.73 | 18 |
| Cf | stock | 2 | 15.51 [15.33-15.82] | 11.45 [11.37-11.58] | 13.12 [12.91-13.27] | 76 | 1279 | 33.1 / 0.4 | 1038 | 22.49 | 18 |
| Cf | stock | 4 | 14.85 [9.60-15.70] | 11.12 [8.71-11.64] | 12.92 [9.49-13.20] | 77 | 1299 | 36.7 / 0.2 | 1036 | 22.03 | 18 |
| Cf | stock | 8 | 14.93 [9.37-15.16] | 11.12 [8.62-11.28] | 12.87 [9.23-13.05] | 78 | 1303 | 37.5 / 0.2 | 1036 | 21.44 | 18 |
| Cf | h2-batch | 1 | 8.72 [8.50-9.02] | 7.64 [7.56-7.85] | 8.33 [8.02-8.56] | 120 | 2014 | 14.6 / 0.1 | 45 | 14.15 | 18 |
| Cf | h2-batch | 2 | 8.25 [7.78-8.84] | 7.42 [7.26-7.67] | 8.76 [8.12-9.60] | 114 | 1915 | 20.7 / 0.1 | 46 | 14.06 | 18 |
| Cf | h2-batch | 4 | 8.57 [7.97-8.92] | 7.56 [7.34-7.69] | 8.37 [8.02-9.56] | 119 | 2003 | 23.9 / 0.1 | 42 | 14.01 | 18 |
| Cf | h2-batch | 8 | 8.61 [8.33-8.94] | 7.61 [7.49-7.73] | 8.39 [7.93-8.75] | 119 | 2000 | 23.8 / 0.1 | 41 | 13.95 | 18 |
| Cf-cb | stock | 1 | 14.65 [13.81-15.45] | 10.98 [10.56-11.21] | 12.82 [12.42-13.07] | 78 | 1308 | 15.4 / 0.4 | 1033 | 21.31 | 18 |
| Cf-cb | stock | 2 | 15.30 [9.47-15.43] | 11.18 [8.68-11.38] | 12.96 [9.40-13.10] | 77 | 1295 | 44.4 / 0.3 | 1046 | 21.63 | 18 |
| Cf-cb | stock | 4 | 15.26 [9.63-15.54] | 11.30 [8.77-11.47] | 13.03 [9.21-13.19] | 77 | 1287 | 55.7 / 0.2 | 1041 | 21.89 | 18 |
| Cf-cb | stock | 8 | 15.55 [15.34-15.70] | 11.42 [11.27-11.57] | 13.07 [12.94-13.22] | 77 | 1284 | 55.8 / 0.2 | 1038 | 22.04 | 18 |
| Cf-cb | h2-batch | 1 | 8.60 [7.94-8.91] | 7.60 [7.44-7.84] | 8.59 [8.06-10.49] | 116 | 1952 | 21.0 / 0.1 | 47 | 14.28 | 18 |
| Cf-cb | h2-batch | 2 | 8.68 [7.90-9.10] | 7.71 [7.31-7.88] | 8.48 [7.85-9.70] | 118 | 1978 | 33.4 / 0.0 | 50 | 14.11 | 18 |
| Cf-cb | h2-batch | 4 | 8.02 [7.84-8.97] | 7.42 [7.30-7.78] | 9.50 [8.22-9.73] | 105 | 1766 | 54.3 / 0.1 | 69 | 12.98 | 18 |
| Cf-cb | h2-batch | 8 | 8.32 [7.85-9.13] | 7.61 [7.27-7.92] | 9.07 [7.88-9.56] | 110 | 1851 | 42.8 / 0.1 | 51 | 13.84 | 18 |

## d16k16

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 15.06 [9.06-16.25] | 11.05 [8.40-12.03] | 12.78 [9.23-13.16] | 78 | 1312 | 30.9 / 0.2 | 1035 | 22.55 | 15 |
| A | stock | 2 | 15.35 [15.00-16.03] | 11.39 [11.00-12.04] | 12.98 [12.77-13.11] | 77 | 1293 | 32.4 / 0.2 | 1034 | 22.59 | 15 |
| A | stock | 4 | 14.93 [12.92-17.78] | 11.15 [10.26-13.46] | 12.93 [12.08-13.96] | 77 | 1298 | 30.8 / 0.2 | 1034 | 21.33 | 15 |
| A | stock | 8 | 13.82 [9.41-16.22] | 10.75 [8.64-12.09] | 12.53 [9.37-13.15] | 80 | 1339 | 29.4 / 0.1 | 1035 | 21.17 | 15 |
| A | h2-batch | 1 | 15.44 [14.54-17.52] | 11.56 [10.87-13.02] | 13.11 [12.84-13.93] | 76 | 1280 | 32.6 / 0.2 | 1034 | 22.36 | 15 |
| A | h2-batch | 2 | 14.71 [9.40-17.20] | 10.84 [8.66-12.89] | 12.84 [9.14-13.25] | 78 | 1306 | 31.2 / 0.1 | 1036 | 21.05 | 15 |
| A | h2-batch | 4 | 15.52 [14.79-17.01] | 11.60 [11.07-12.56] | 13.09 [12.89-13.87] | 76 | 1282 | 32.0 / 0.2 | 1034 | 22.50 | 15 |
| A | h2-batch | 8 | 14.92 [14.00-16.96] | 11.24 [10.65-12.76] | 12.90 [12.52-13.20] | 78 | 1301 | 31.4 / 0.2 | 1034 | 21.82 | 15 |
| Cf | stock | 1 | 15.08 [9.04-15.41] | 10.91 [8.39-11.27] | 12.80 [8.81-12.98] | 78 | 1310 | 10.8 / 0.4 | 1036 | 21.42 | 15 |
| Cf | stock | 2 | 15.49 [15.38-16.20] | 11.37 [11.30-11.78] | 13.00 [12.88-13.49] | 77 | 1291 | 33.1 / 0.4 | 1039 | 22.53 | 15 |
| Cf | stock | 4 | 14.61 [14.20-15.53] | 11.03 [10.87-11.46] | 12.80 [12.54-13.06] | 78 | 1311 | 37.0 / 0.3 | 1036 | 22.27 | 15 |
| Cf | stock | 8 | 14.55 [9.44-15.57] | 10.89 [8.79-11.48] | 12.86 [9.26-13.65] | 78 | 1305 | 37.6 / 0.4 | 1036 | 21.71 | 15 |
| Cf | h2-batch | 1 | 8.56 [8.21-8.80] | 7.38 [7.15-7.58] | 8.17 [7.94-8.44] | 122 | 2053 | 14.6 / 0.1 | 44 | 14.72 | 15 |
| Cf | h2-batch | 2 | 8.85 [8.21-8.97] | 7.63 [7.36-8.01] | 8.01 [7.84-8.48] | 125 | 2094 | 19.2 / 0.1 | 42 | 14.03 | 15 |
| Cf | h2-batch | 4 | 8.45 [8.10-8.87] | 7.48 [7.26-7.90] | 8.38 [8.21-8.60] | 119 | 2002 | 23.9 / 0.2 | 41 | 14.20 | 15 |
| Cf | h2-batch | 8 | 8.47 [8.23-9.13] | 7.55 [7.38-7.89] | 8.38 [7.73-8.65] | 119 | 2003 | 23.0 / 0.3 | 40 | 13.96 | 15 |
| Cf-cb | stock | 1 | 15.22 [13.60-15.54] | 11.01 [10.49-11.39] | 12.85 [12.02-13.08] | 78 | 1305 | 12.7 / 0.3 | 1033 | 21.20 | 15 |
| Cf-cb | stock | 2 | 15.62 [15.10-15.98] | 11.47 [11.12-11.74] | 13.09 [12.89-13.58] | 76 | 1282 | 41.5 / 0.3 | 1045 | 22.42 | 15 |
| Cf-cb | stock | 4 | 15.14 [9.24-15.41] | 11.27 [8.66-11.48] | 12.93 [9.56-13.10] | 77 | 1297 | 52.6 / 0.2 | 1042 | 22.39 | 15 |
| Cf-cb | stock | 8 | 15.40 [9.18-16.05] | 11.27 [8.50-11.70] | 12.95 [9.06-13.72] | 77 | 1296 | 53.0 / 0.2 | 1040 | 21.32 | 15 |
| Cf-cb | h2-batch | 1 | 8.52 [8.15-8.66] | 7.57 [7.37-7.72] | 8.43 [8.14-8.64] | 119 | 1991 | 18.4 / 0.1 | 45 | 14.49 | 15 |
| Cf-cb | h2-batch | 2 | 8.70 [8.18-9.05] | 7.66 [7.35-8.20] | 8.37 [7.85-8.67] | 119 | 2004 | 31.8 / 0.1 | 49 | 14.19 | 15 |
| Cf-cb | h2-batch | 4 | 8.67 [8.22-9.28] | 7.83 [7.47-8.10] | 8.42 [7.99-8.96] | 119 | 1994 | 36.8 / 0.1 | 47 | 14.01 | 15 |
| Cf-cb | h2-batch | 8 | 8.77 [8.30-9.21] | 7.70 [7.35-8.31] | 8.29 [7.79-8.97] | 121 | 2024 | 37.5 / 0.1 | 46 | 14.14 | 15 |
| Cf-m4 | stock | 1 | 18.50 [17.54-19.96] | 14.11 [13.34-14.96] | 4.80 [3.87-5.12] | 208 | 3498 | 11.9 / 0.6 | 1031 | 23.16 | 15 |
| Cf-m4 | stock | 2 | 20.22 [19.13-21.74] | 14.90 [14.10-15.80] | 4.74 [4.50-5.18] | 211 | 3539 | 37.4 / 0.7 | 1036 | 23.72 | 15 |
| Cf-m4 | stock | 4 | 20.33 [17.39-21.26] | 14.90 [13.64-15.52] | 4.78 [4.48-4.97] | 209 | 3514 | 44.1 / 0.8 | 1032 | 23.72 | 15 |
| Cf-m4 | stock | 8 | 20.80 [19.18-21.53] | 15.31 [14.79-15.68] | 4.80 [4.67-4.90] | 208 | 3495 | 46.8 / 0.7 | 1031 | 23.78 | 15 |
| Cf-m4 | h2-batch | 1 | 12.02 [11.00-12.34] | 10.80 [9.98-11.05] | 4.12 [3.74-4.24] | 243 | 4077 | 18.4 / 0.4 | 54 | 19.95 | 15 |
| Cf-m4 | h2-batch | 2 | 12.15 [11.59-12.81] | 11.18 [10.67-11.66] | 3.92 [3.72-4.02] | 255 | 4277 | 26.4 / 0.6 | 56 | 19.93 | 15 |
| Cf-m4 | h2-batch | 4 | 12.31 [11.87-13.40] | 11.13 [10.80-12.24] | 3.89 [3.72-4.02] | 257 | 4317 | 29.8 / 0.7 | 54 | 20.05 | 15 |
| Cf-m4 | h2-batch | 8 | 12.26 [11.73-12.64] | 11.16 [10.58-11.63] | 3.95 [3.86-4.05] | 253 | 4247 | 28.6 / 0.6 | 52 | 19.93 | 15 |
| Cf-cb-m4 | stock | 1 | 19.41 [17.69-21.42] | 14.36 [13.53-15.83] | 4.73 [4.05-4.86] | 211 | 3545 | 13.1 / 0.5 | 1028 | 23.58 | 15 |
| Cf-cb-m4 | stock | 2 | 20.16 [18.23-21.92] | 14.81 [14.06-16.08] | 4.77 [4.43-4.99] | 210 | 3520 | 46.7 / 0.9 | 1044 | 23.74 | 15 |
| Cf-cb-m4 | stock | 4 | 20.38 [17.76-21.24] | 15.07 [13.78-15.77] | 4.82 [4.39-5.14] | 207 | 3478 | 59.5 / 1.9 | 1037 | 23.57 | 15 |
| Cf-cb-m4 | stock | 8 | 20.30 [19.76-21.68] | 15.00 [14.62-16.26] | 4.78 [4.65-4.88] | 209 | 3511 | 61.1 / 1.4 | 1034 | 23.70 | 15 |
| Cf-cb-m4 | h2-batch | 1 | 12.61 [11.95-13.01] | 11.33 [10.72-11.52] | 3.87 [3.55-4.15] | 258 | 4331 | 21.6 / 0.2 | 55 | 20.56 | 15 |
| Cf-cb-m4 | h2-batch | 2 | 13.38 [12.85-13.73] | 11.91 [11.45-12.45] | 3.49 [3.38-3.78] | 286 | 4805 | 32.6 / 1.0 | 54 | 20.74 | 15 |
| Cf-cb-m4 | h2-batch | 4 | 13.31 [12.82-14.06] | 11.75 [11.57-12.71] | 3.55 [3.41-3.70] | 281 | 4720 | 40.6 / 1.1 | 51 | 20.62 | 15 |
| Cf-cb-m4 | h2-batch | 8 | 13.15 [12.30-13.90] | 11.74 [11.13-12.54] | 3.61 [3.39-4.01] | 277 | 4646 | 40.3 / 1.0 | 50 | 20.55 | 15 |

## d16k32

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 14.87 [9.54-16.12] | 11.11 [8.70-12.45] | 12.96 [9.34-13.21] | 77 | 1294 | 30.8 / 0.2 | 1035 | 22.21 | 12 |
| A | stock | 2 | 15.38 [14.58-15.92] | 11.43 [10.86-12.00] | 12.96 [12.86-13.46] | 77 | 1294 | 31.5 / 0.2 | 1034 | 22.84 | 12 |
| A | stock | 4 | 12.33 [9.44-16.07] | 9.89 [8.69-11.87] | 11.36 [9.30-13.02] | 88 | 1477 | 30.9 / 0.1 | 1037 | 19.34 | 12 |
| A | stock | 8 | 15.12 [8.77-16.05] | 11.22 [8.13-12.01] | 12.96 [10.30-13.24] | 77 | 1294 | 31.5 / 0.1 | 1034 | 22.92 | 12 |
| A | h2-batch | 1 | 15.01 [14.05-15.55] | 11.16 [10.64-11.67] | 12.94 [12.66-13.10] | 77 | 1297 | 31.0 / 0.2 | 1034 | 22.66 | 12 |
| A | h2-batch | 2 | 15.44 [14.29-17.13] | 11.44 [10.70-12.97] | 13.07 [12.72-13.47] | 77 | 1284 | 31.8 / 0.2 | 1034 | 22.69 | 12 |
| A | h2-batch | 4 | 10.35 [9.01-16.21] | 9.55 [8.25-12.09] | 9.39 [9.04-13.03] | 106 | 1786 | 27.2 / 0.1 | 1038 | 15.77 | 12 |
| A | h2-batch | 8 | 15.30 [14.48-16.68] | 11.20 [10.62-12.42] | 13.16 [12.24-13.28] | 76 | 1275 | 31.9 / 0.1 | 1034 | 22.20 | 12 |
| Cf | stock | 1 | 15.05 [9.05-15.45] | 11.00 [8.45-11.38] | 12.80 [8.83-13.03] | 78 | 1311 | 10.6 / 0.5 | 1035 | 21.88 | 12 |
| Cf | stock | 2 | 15.51 [15.20-15.79] | 11.33 [11.14-11.55] | 13.05 [12.88-13.21] | 77 | 1285 | 32.6 / 0.4 | 1039 | 23.35 | 12 |
| Cf | stock | 4 | 15.45 [9.03-15.53] | 11.36 [8.43-11.49] | 12.89 [9.65-13.01] | 78 | 1301 | 38.7 / 0.3 | 1037 | 22.74 | 12 |
| Cf | stock | 8 | 15.49 [15.27-15.65] | 11.38 [11.27-11.49] | 13.01 [12.97-13.13] | 77 | 1289 | 39.2 / 0.4 | 1036 | 23.05 | 12 |
| Cf | h2-batch | 1 | 7.53 [7.41-8.03] | 7.01 [6.89-7.28] | 9.26 [8.64-9.41] | 108 | 1812 | 29.4 / 0.2 | 65 | 13.69 | 12 |
| Cf | h2-batch | 2 | 8.72 [7.65-8.95] | 7.63 [7.09-7.78] | 8.35 [7.92-9.38] | 120 | 2010 | 19.4 / 0.1 | 41 | 14.31 | 12 |
| Cf | h2-batch | 4 | 8.06 [7.75-8.98] | 7.52 [7.30-7.84] | 9.29 [8.01-9.52] | 108 | 1807 | 33.7 / 0.2 | 63 | 14.02 | 12 |
| Cf | h2-batch | 8 | 8.63 [7.83-9.03] | 7.66 [7.27-7.81] | 8.42 [7.87-9.74] | 119 | 1993 | 22.9 / 0.3 | 40 | 14.44 | 12 |
| Cf-cb | stock | 1 | 15.44 [14.70-15.74] | 11.28 [10.93-11.64] | 12.92 [12.76-13.09] | 77 | 1299 | 11.2 / 0.4 | 1033 | 21.74 | 12 |
| Cf-cb | stock | 2 | 15.60 [9.08-15.81] | 11.41 [8.44-11.51] | 13.05 [9.21-13.15] | 77 | 1285 | 40.8 / 0.3 | 1045 | 22.33 | 12 |
| Cf-cb | stock | 4 | 15.63 [15.43-16.07] | 11.55 [11.39-11.71] | 13.00 [12.90-13.67] | 77 | 1290 | 51.6 / 0.4 | 1040 | 23.10 | 12 |
| Cf-cb | stock | 8 | 15.57 [9.28-15.99] | 11.48 [8.65-11.84] | 13.04 [9.07-13.34] | 77 | 1287 | 50.4 / 0.3 | 1040 | 22.39 | 12 |
| Cf-cb | h2-batch | 1 | 8.85 [8.45-8.97] | 7.67 [7.59-7.85] | 8.11 [7.74-8.41] | 123 | 2069 | 16.8 / 0.1 | 44 | 14.54 | 12 |
| Cf-cb | h2-batch | 2 | 8.38 [7.85-8.94] | 7.61 [7.38-7.99] | 8.61 [7.87-9.42] | 116 | 1949 | 29.6 / 0.2 | 48 | 14.47 | 12 |
| Cf-cb | h2-batch | 4 | 8.97 [7.89-9.29] | 7.91 [7.40-8.32] | 8.03 [7.91-10.54] | 125 | 2089 | 35.9 / 0.2 | 44 | 14.40 | 12 |
| Cf-cb | h2-batch | 8 | 8.75 [7.83-9.00] | 7.78 [7.35-8.04] | 8.18 [7.91-10.12] | 122 | 2052 | 37.1 / 0.3 | 45 | 14.35 | 12 |
| Cf-m4 | stock | 1 | 18.52 [17.85-19.12] | 14.07 [13.71-14.37] | 4.40 [3.97-4.60] | 227 | 3817 | 11.8 / 0.7 | 1034 | 24.48 | 12 |
| Cf-m4 | stock | 2 | 21.05 [19.88-21.93] | 15.50 [14.76-16.00] | 4.79 [4.67-4.85] | 209 | 3504 | 37.3 / 0.9 | 1038 | 24.43 | 12 |
| Cf-m4 | stock | 4 | 20.79 [20.18-21.58] | 15.47 [14.82-15.96] | 4.75 [4.69-4.92] | 211 | 3532 | 44.7 / 1.0 | 1033 | 24.40 | 12 |
| Cf-m4 | stock | 8 | 21.46 [20.10-21.93] | 15.77 [14.88-16.01] | 4.77 [4.66-4.88] | 210 | 3517 | 45.6 / 1.4 | 1032 | 24.47 | 12 |
| Cf-m4 | h2-batch | 1 | 12.63 [11.96-13.22] | 11.31 [10.82-11.64] | 3.78 [3.40-3.88] | 265 | 4444 | 17.5 / 0.5 | 51 | 20.86 | 12 |
| Cf-m4 | h2-batch | 2 | 12.47 [12.13-12.71] | 11.22 [11.00-11.37] | 3.88 [3.57-4.07] | 257 | 4319 | 24.1 / 0.5 | 49 | 20.41 | 12 |
| Cf-m4 | h2-batch | 4 | 12.90 [12.38-13.39] | 11.56 [10.96-11.82] | 3.58 [3.43-3.83] | 279 | 4689 | 23.8 / 0.9 | 43 | 20.90 | 12 |
| Cf-m4 | h2-batch | 8 | 12.96 [12.47-13.30] | 11.48 [11.32-11.84] | 3.55 [3.46-3.90] | 282 | 4728 | 23.7 / 0.8 | 42 | 20.96 | 12 |
| Cf-cb-m4 | stock | 1 | 19.28 [18.45-20.32] | 14.26 [13.95-15.25] | 4.69 [4.18-4.76] | 213 | 3576 | 11.3 / 0.4 | 1031 | 24.39 | 12 |
| Cf-cb-m4 | stock | 2 | 20.87 [20.03-22.03] | 15.40 [14.82-16.57] | 4.71 [4.52-4.89] | 212 | 3563 | 43.2 / 1.8 | 1044 | 24.48 | 12 |
| Cf-cb-m4 | stock | 4 | 20.61 [19.65-21.45] | 15.26 [14.60-16.24] | 4.67 [4.33-4.81] | 214 | 3593 | 55.7 / 2.4 | 1038 | 24.31 | 12 |
| Cf-cb-m4 | stock | 8 | 21.30 [20.10-21.61] | 15.57 [14.85-16.14] | 4.71 [4.54-4.84] | 212 | 3561 | 57.7 / 1.5 | 1035 | 24.38 | 12 |
| Cf-cb-m4 | h2-batch | 1 | 13.10 [12.32-13.68] | 11.50 [10.81-12.07] | 3.53 [3.39-4.04] | 283 | 4756 | 17.0 / 0.4 | 47 | 21.25 | 12 |
| Cf-cb-m4 | h2-batch | 2 | 13.45 [13.06-14.66] | 11.84 [11.47-13.23] | 3.53 [3.39-4.09] | 283 | 4755 | 28.3 / 1.4 | 50 | 20.86 | 12 |
| Cf-cb-m4 | h2-batch | 4 | 13.50 [12.73-14.24] | 11.93 [11.28-12.67] | 3.49 [3.43-3.65] | 286 | 4803 | 36.8 / 1.7 | 47 | 20.95 | 12 |
| Cf-cb-m4 | h2-batch | 8 | 13.32 [12.51-15.15] | 11.88 [11.13-13.56] | 3.89 [3.42-3.95] | 257 | 4309 | 39.1 / 2.1 | 51 | 20.72 | 12 |

## c54k1

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 3.75 [3.72-3.79] | 2.57 [2.55-2.60] | 4.11 [4.07-4.15] | 244 |  | 11.1 / 0.1 | 258 | 2.87 | 24 |
| A | stock | 2 | 3.76 [3.75-3.87] | 2.59 [2.57-2.70] | 4.11 [4.09-4.24] | 243 |  | 11.0 / 0.1 | 258 | 2.88 | 24 |
| A | stock | 4 | 3.78 [3.73-3.85] | 2.58 [2.56-2.69] | 4.13 [4.09-4.21] | 242 |  | 11.1 / 0.1 | 258 | 2.87 | 24 |
| A | stock | 8 | 3.76 [3.73-3.90] | 2.58 [2.55-2.73] | 4.10 [4.06-4.25] | 244 |  | 11.2 / 0.1 | 258 | 2.87 | 24 |
| A | h2-batch | 1 | 3.79 [3.71-3.85] | 2.59 [2.56-2.68] | 4.13 [4.06-4.19] | 242 |  | 11.0 / 0.1 | 258 | 2.87 | 24 |
| A | h2-batch | 2 | 3.75 [3.72-3.89] | 2.57 [2.54-2.68] | 4.09 [4.06-4.24] | 245 |  | 11.1 / 0.1 | 258 | 2.86 | 24 |
| A | h2-batch | 4 | 3.77 [3.73-3.82] | 2.59 [2.57-2.64] | 4.11 [4.09-4.15] | 243 |  | 11.1 / 0.1 | 258 | 2.88 | 24 |
| A | h2-batch | 8 | 3.75 [3.69-3.86] | 2.57 [2.56-2.66] | 4.10 [4.05-4.22] | 244 |  | 11.1 / 0.1 | 259 | 2.86 | 24 |
| Cf | stock | 1 | 3.76 [3.71-3.80] | 2.54 [2.52-2.56] | 4.25 [4.19-4.31] | 235 |  | 5.1 / 0.1 | 260 | 2.87 | 24 |
| Cf | stock | 2 | 3.84 [3.81-3.91] | 2.65 [2.62-2.68] | 4.25 [4.22-4.31] | 235 |  | 14.9 / 0.1 | 263 | 2.89 | 24 |
| Cf | stock | 4 | 3.85 [3.79-3.89] | 2.67 [2.63-2.68] | 4.25 [4.19-4.29] | 235 |  | 15.2 / 0.1 | 260 | 2.88 | 24 |
| Cf | stock | 8 | 3.86 [3.81-3.91] | 2.66 [2.63-2.70] | 4.26 [4.22-4.31] | 235 |  | 15.0 / 0.1 | 260 | 2.87 | 24 |
| Cf | h2-batch | 1 | 1.75 [1.70-1.79] | 1.59 [1.55-1.64] | 3.08 [2.99-3.74] | 325 |  | 7.3 / 0.0 | 20 | 2.20 | 24 |
| Cf | h2-batch | 2 | 1.82 [1.79-1.93] | 1.67 [1.65-1.72] | 3.11 [3.03-4.36] | 322 |  | 13.2 / 0.0 | 22 | 2.22 | 24 |
| Cf | h2-batch | 4 | 1.83 [1.80-2.01] | 1.68 [1.66-1.75] | 3.85 [3.04-7.70] | 260 |  | 11.1 / 0.0 | 20 | 3.07 | 24 |
| Cf | h2-batch | 8 | 1.84 [1.80-1.89] | 1.70 [1.67-1.75] | 3.22 [3.10-3.40] | 311 |  | 14.2 / 0.0 | 20 | 2.34 | 24 |
| Cf-cb | stock | 1 | 3.77 [3.72-3.81] | 2.54 [2.52-2.56] | 4.25 [4.21-4.30] | 235 |  | 4.2 / 0.1 | 259 | 2.88 | 24 |
| Cf-cb | stock | 2 | 3.85 [3.78-3.89] | 2.65 [2.63-2.67] | 4.25 [4.21-4.32] | 235 |  | 13.1 / 0.1 | 262 | 2.89 | 24 |
| Cf-cb | stock | 4 | 3.85 [3.80-3.91] | 2.66 [2.63-2.70] | 4.25 [4.18-4.31] | 235 |  | 13.2 / 0.1 | 259 | 2.87 | 24 |
| Cf-cb | stock | 8 | 3.92 [3.86-3.99] | 2.69 [2.65-2.74] | 4.28 [4.23-4.36] | 233 |  | 14.2 / 0.1 | 260 | 2.89 | 24 |
| Cf-cb | h2-batch | 1 | 1.81 [1.75-1.90] | 1.64 [1.60-1.67] | 3.68 [3.04-5.49] | 272 |  | 5.4 / 0.0 | 19 | 2.96 | 24 |
| Cf-cb | h2-batch | 2 | 1.98 [1.93-2.09] | 1.73 [1.71-1.76] | 3.15 [2.99-3.86] | 317 |  | 8.1 / 0.0 | 20 | 2.35 | 24 |
| Cf-cb | h2-batch | 4 | 1.84 [1.80-2.03] | 1.69 [1.66-1.79] | 3.26 [3.15-4.29] | 307 |  | 9.8 / 0.0 | 20 | 2.36 | 24 |
| Cf-cb | h2-batch | 8 | 1.86 [1.82-2.09] | 1.71 [1.67-1.77] | 3.72 [3.06-4.70] | 269 |  | 9.2 / 0.0 | 19 | 2.95 | 24 |

## c54k8

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 3.30 [2.41-3.85] | 2.64 [2.20-2.92] | 3.46 [2.94-3.65] | 289 |  | 9.0 / 0.0 | 259 | 6.24 | 18 |
| A | stock | 2 | 3.54 [3.26-4.21] | 2.67 [2.53-3.17] | 3.51 [3.38-3.67] | 285 |  | 9.6 / 0.0 | 258 | 6.47 | 18 |
| A | stock | 4 | 2.75 [2.27-3.55] | 2.45 [2.04-2.82] | 3.14 [2.93-3.58] | 318 |  | 9.3 / 0.0 | 259 | 5.90 | 18 |
| A | stock | 8 | 3.67 [3.46-4.02] | 2.73 [2.55-3.09] | 3.55 [3.46-3.65] | 282 |  | 9.3 / 0.0 | 258 | 6.97 | 18 |
| A | h2-batch | 1 | 2.46 [2.28-3.53] | 2.28 [2.04-2.80] | 3.14 [2.93-3.45] | 318 |  | 9.3 / 0.0 | 259 | 5.79 | 18 |
| A | h2-batch | 2 | 3.61 [2.44-3.92] | 2.65 [2.10-3.18] | 3.36 [2.90-3.69] | 298 |  | 9.1 / 0.0 | 259 | 5.93 | 18 |
| A | h2-batch | 4 | 3.67 [3.46-4.21] | 2.73 [2.57-3.23] | 3.58 [3.52-3.75] | 279 |  | 9.5 / 0.0 | 259 | 7.10 | 18 |
| A | h2-batch | 8 | 3.71 [2.51-3.81] | 2.73 [2.24-2.85] | 3.52 [3.07-3.64] | 284 |  | 9.6 / 0.0 | 258 | 6.56 | 18 |
| Cf | stock | 1 | 3.60 [2.82-4.00] | 2.87 [2.50-3.08] | 3.53 [3.12-3.72] | 283 |  | 5.5 / 0.1 | 258 | 7.23 | 18 |
| Cf | stock | 2 | 3.95 [3.87-4.04] | 3.06 [3.00-3.10] | 3.62 [3.53-3.71] | 277 |  | 10.9 / 0.1 | 259 | 7.08 | 18 |
| Cf | stock | 4 | 3.69 [3.54-3.99] | 2.95 [2.85-3.08] | 3.60 [3.50-3.69] | 278 |  | 12.2 / 0.0 | 259 | 7.25 | 18 |
| Cf | stock | 8 | 4.05 [3.69-4.09] | 3.10 [2.95-3.16] | 3.69 [3.60-3.75] | 271 |  | 12.7 / 0.1 | 258 | 7.54 | 18 |
| Cf | h2-batch | 1 | 2.25 [2.18-2.33] | 2.03 [2.01-2.10] | 2.97 [2.73-3.22] | 337 |  | 6.4 / 0.0 | 13 | 5.70 | 18 |
| Cf | h2-batch | 2 | 2.24 [2.17-2.31] | 2.04 [1.97-2.12] | 2.83 [2.77-3.03] | 353 |  | 8.5 / 0.0 | 12 | 5.52 | 18 |
| Cf | h2-batch | 4 | 2.22 [2.18-2.37] | 2.04 [1.99-2.10] | 2.86 [2.73-3.09] | 350 |  | 9.8 / 0.0 | 12 | 5.67 | 18 |
| Cf | h2-batch | 8 | 2.25 [2.20-2.31] | 2.07 [2.02-2.12] | 2.91 [2.77-3.10] | 344 |  | 10.3 / 0.0 | 12 | 5.39 | 18 |
| Cf-cb | stock | 1 | 3.37 [3.15-3.53] | 2.49 [2.38-2.55] | 3.62 [3.48-3.73] | 276 |  | 5.0 / 0.1 | 257 | 6.99 | 18 |
| Cf-cb | stock | 2 | 3.14 [2.99-3.63] | 2.42 [2.36-2.65] | 3.46 [3.40-3.59] | 289 |  | 10.6 / 0.1 | 260 | 6.66 | 18 |
| Cf-cb | stock | 4 | 3.29 [3.11-3.63] | 2.54 [2.47-2.70] | 3.57 [3.41-3.68] | 280 |  | 13.9 / 0.0 | 259 | 7.20 | 18 |
| Cf-cb | stock | 8 | 3.67 [2.41-3.80] | 2.71 [2.11-2.79] | 3.62 [2.94-3.71] | 277 |  | 14.3 / 0.0 | 259 | 6.95 | 18 |
| Cf-cb | h2-batch | 1 | 1.73 [1.71-1.79] | 1.58 [1.56-1.61] | 2.89 [2.67-3.24] | 346 |  | 6.2 / 0.0 | 15 | 4.90 | 18 |
| Cf-cb | h2-batch | 2 | 1.97 [1.78-2.02] | 1.74 [1.67-1.79] | 2.82 [2.60-3.48] | 354 |  | 8.9 / 0.0 | 14 | 5.14 | 18 |
| Cf-cb | h2-batch | 4 | 1.82 [1.78-1.94] | 1.71 [1.67-1.75] | 2.92 [2.76-3.10] | 343 |  | 14.1 / 0.0 | 18 | 5.14 | 18 |
| Cf-cb | h2-batch | 8 | 1.86 [1.77-1.99] | 1.70 [1.65-1.75] | 2.95 [2.78-3.06] | 339 |  | 13.9 / 0.0 | 19 | 5.29 | 18 |

## c54k16

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 2.99 [2.36-4.20] | 2.62 [2.19-3.29] | 2.74 [2.59-3.46] | 364 |  | 8.7 / 0.0 | 259 | 4.90 | 18 |
| A | stock | 2 | 3.65 [2.35-4.77] | 2.79 [2.12-3.62] | 3.43 [2.63-3.66] | 292 |  | 8.5 / 0.0 | 259 | 6.69 | 18 |
| A | stock | 4 | 3.84 [3.47-4.53] | 2.89 [2.63-3.59] | 3.44 [3.37-3.57] | 290 |  | 8.8 / 0.0 | 258 | 7.25 | 18 |
| A | stock | 8 | 4.07 [3.52-4.87] | 3.18 [2.60-3.66] | 3.50 [3.40-3.80] | 286 |  | 9.0 / 0.1 | 258 | 7.12 | 18 |
| A | h2-batch | 1 | 3.69 [2.62-4.52] | 2.86 [2.46-3.55] | 3.39 [2.91-3.72] | 295 |  | 9.5 / 0.0 | 259 | 6.73 | 18 |
| A | h2-batch | 2 | 3.76 [3.54-4.59] | 2.93 [2.62-3.59] | 3.46 [3.35-3.63] | 289 |  | 8.7 / 0.0 | 259 | 7.11 | 18 |
| A | h2-batch | 4 | 3.94 [3.44-4.54] | 3.06 [2.59-3.54] | 3.42 [3.34-3.65] | 292 |  | 8.8 / 0.0 | 259 | 6.95 | 18 |
| A | h2-batch | 8 | 3.40 [2.60-3.97] | 2.83 [2.40-3.10] | 3.28 [2.71-3.45] | 305 |  | 8.4 / 0.0 | 259 | 6.44 | 18 |
| Cf | stock | 1 | 3.90 [2.58-4.03] | 2.97 [2.38-3.47] | 3.44 [2.83-3.56] | 291 |  | 5.0 / 0.1 | 258 | 7.56 | 18 |
| Cf | stock | 2 | 3.66 [2.61-3.89] | 2.88 [2.38-3.59] | 3.46 [2.77-3.58] | 289 |  | 10.2 / 0.1 | 259 | 7.38 | 18 |
| Cf | stock | 4 | 3.82 [2.61-4.01] | 2.98 [2.40-3.48] | 3.37 [2.84-3.57] | 297 |  | 11.8 / 0.1 | 259 | 7.45 | 18 |
| Cf | stock | 8 | 3.83 [3.53-4.77] | 2.97 [2.81-4.06] | 3.48 [3.35-3.60] | 287 |  | 11.4 / 0.1 | 258 | 7.55 | 18 |
| Cf | h2-batch | 1 | 2.18 [2.13-3.27] | 2.01 [1.96-3.08] | 2.74 [2.59-2.84] | 366 |  | 5.7 / 0.0 | 13 | 5.50 | 18 |
| Cf | h2-batch | 2 | 2.37 [2.29-3.39] | 2.08 [2.00-3.17] | 2.63 [2.34-2.76] | 381 |  | 7.7 / 0.0 | 11 | 5.56 | 18 |
| Cf | h2-batch | 4 | 2.24 [2.16-3.42] | 2.03 [1.95-3.20] | 2.61 [2.39-2.75] | 383 |  | 8.7 / 0.1 | 11 | 5.27 | 18 |
| Cf | h2-batch | 8 | 2.24 [2.12-3.22] | 2.04 [1.98-3.09] | 2.77 [2.57-3.16] | 360 |  | 12.2 / 0.1 | 18 | 5.30 | 18 |
| Cf-cb | stock | 1 | 3.05 [2.16-3.32] | 2.35 [1.90-2.83] | 3.36 [2.62-3.50] | 297 |  | 4.8 / 0.1 | 258 | 6.74 | 18 |
| Cf-cb | stock | 2 | 3.24 [2.14-3.65] | 2.47 [1.95-3.15] | 3.35 [2.78-3.44] | 298 |  | 10.3 / 0.0 | 260 | 6.76 | 18 |
| Cf-cb | stock | 4 | 3.48 [2.19-3.73] | 2.62 [1.98-3.10] | 3.43 [2.81-3.65] | 291 |  | 13.7 / 0.0 | 259 | 7.27 | 18 |
| Cf-cb | stock | 8 | 3.44 [3.10-4.39] | 2.59 [2.44-3.51] | 3.44 [3.33-3.62] | 290 |  | 13.1 / 0.0 | 259 | 7.32 | 18 |
| Cf-cb | h2-batch | 1 | 1.88 [1.82-2.87] | 1.59 [1.56-2.56] | 2.35 [2.23-2.55] | 426 |  | 5.3 / 0.0 | 12 | 4.98 | 18 |
| Cf-cb | h2-batch | 2 | 1.82 [1.71-2.74] | 1.64 [1.55-2.56] | 2.67 [2.31-3.23] | 374 |  | 8.7 / 0.0 | 14 | 5.20 | 18 |
| Cf-cb | h2-batch | 4 | 1.85 [1.74-2.78] | 1.66 [1.61-2.67] | 2.70 [2.45-2.86] | 370 |  | 12.4 / 0.0 | 15 | 4.78 | 18 |
| Cf-cb | h2-batch | 8 | 1.99 [1.81-2.74] | 1.71 [1.67-2.62] | 2.50 [2.32-2.78] | 400 |  | 11.4 / 0.0 | 12 | 4.94 | 18 |
| Cf-m4 | stock | 1 | 4.69 [4.40-5.91] | 3.61 [3.46-4.87] | 1.44 [1.36-1.64] | 696 |  | 5.8 / 0.1 | 258 | 6.99 | 18 |
| Cf-m4 | stock | 2 | 4.87 [4.63-5.68] | 3.79 [3.65-4.77] | 1.41 [1.35-1.64] | 707 |  | 11.7 / 0.2 | 260 | 7.20 | 18 |
| Cf-m4 | stock | 4 | 5.13 [4.53-5.96] | 3.94 [3.61-4.93] | 1.48 [1.41-1.60] | 675 |  | 13.6 / 0.2 | 259 | 7.26 | 18 |
| Cf-m4 | stock | 8 | 5.11 [4.54-5.87] | 3.90 [3.63-4.87] | 1.46 [1.38-1.53] | 686 |  | 13.7 / 0.2 | 259 | 7.28 | 18 |
| Cf-m4 | h2-batch | 1 | 2.93 [2.76-3.92] | 2.65 [2.47-3.64] | 1.32 [1.22-1.41] | 758 |  | 7.2 / 0.1 | 15 | 6.34 | 18 |
| Cf-m4 | h2-batch | 2 | 2.90 [2.81-3.99] | 2.65 [2.59-3.77] | 1.35 [1.28-1.51] | 738 |  | 10.7 / 0.1 | 17 | 6.52 | 18 |
| Cf-m4 | h2-batch | 4 | 3.01 [2.90-4.14] | 2.70 [2.61-3.90] | 1.30 [1.20-1.42] | 767 |  | 10.2 / 0.2 | 15 | 6.67 | 18 |
| Cf-m4 | h2-batch | 8 | 2.92 [2.85-3.95] | 2.69 [2.61-3.78] | 1.41 [1.32-1.51] | 710 |  | 11.0 / 0.2 | 16 | 6.78 | 18 |
| Cf-cb-m4 | stock | 1 | 4.16 [3.75-4.48] | 3.16 [2.92-3.66] | 1.39 [1.30-2.00] | 719 |  | 5.8 / 0.1 | 258 | 5.91 | 18 |
| Cf-cb-m4 | stock | 2 | 4.53 [4.24-4.84] | 3.38 [3.26-3.87] | 1.40 [1.32-1.96] | 712 |  | 12.6 / 0.0 | 260 | 6.59 | 18 |
| Cf-cb-m4 | stock | 4 | 4.40 [4.22-4.86] | 3.35 [3.25-3.99] | 1.45 [1.31-2.02] | 690 |  | 15.2 / 0.1 | 260 | 6.78 | 18 |
| Cf-cb-m4 | stock | 8 | 4.41 [4.13-5.05] | 3.35 [3.23-4.11] | 1.36 [1.30-2.09] | 735 |  | 15.5 / 0.1 | 260 | 6.64 | 18 |
| Cf-cb-m4 | h2-batch | 1 | 2.59 [2.39-3.23] | 2.28 [2.13-2.92] | 1.39 [1.19-1.88] | 722 |  | 6.3 / 0.0 | 19 | 5.55 | 18 |
| Cf-cb-m4 | h2-batch | 2 | 2.69 [2.52-3.22] | 2.37 [2.29-2.99] | 1.32 [1.25-1.92] | 757 |  | 11.1 / 0.0 | 20 | 5.78 | 18 |
| Cf-cb-m4 | h2-batch | 4 | 2.71 [2.51-3.32] | 2.41 [2.27-3.01] | 1.35 [1.20-1.95] | 743 |  | 12.1 / 0.0 | 19 | 5.47 | 18 |
| Cf-cb-m4 | h2-batch | 8 | 2.76 [2.53-3.39] | 2.46 [2.28-3.09] | 1.29 [1.23-1.87] | 777 |  | 12.3 / 0.0 | 20 | 5.99 | 18 |

## c54k32

| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A | stock | 1 | 4.28 [3.83-4.52] | 3.38 [3.02-3.55] | 3.41 [3.27-3.71] | 293 |  | 8.7 / 0.1 | 259 | 7.76 | 15 |
| A | stock | 2 | 4.26 [3.79-4.86] | 3.36 [3.00-3.80] | 3.44 [3.34-3.69] | 290 |  | 8.9 / 0.0 | 258 | 7.15 | 15 |
| A | stock | 4 | 4.12 [3.62-4.80] | 3.25 [2.80-3.93] | 3.41 [3.35-3.62] | 293 |  | 8.6 / 0.1 | 259 | 7.48 | 15 |
| A | stock | 8 | 4.12 [3.86-4.88] | 3.24 [2.91-3.84] | 3.43 [3.37-3.69] | 292 |  | 8.6 / 0.0 | 259 | 7.56 | 15 |
| A | h2-batch | 1 | 4.13 [3.81-4.56] | 3.37 [2.92-3.64] | 3.42 [3.36-3.67] | 292 |  | 8.9 / 0.0 | 258 | 7.30 | 15 |
| A | h2-batch | 2 | 4.29 [3.83-4.88] | 3.43 [3.00-3.84] | 3.49 [3.36-3.63] | 287 |  | 8.7 / 0.0 | 259 | 7.19 | 15 |
| A | h2-batch | 4 | 4.41 [3.83-4.81] | 3.36 [2.99-3.80] | 3.46 [3.32-3.76] | 289 |  | 8.9 / 0.1 | 258 | 7.38 | 15 |
| A | h2-batch | 8 | 4.22 [3.83-4.53] | 3.32 [2.91-3.64] | 3.44 [3.35-3.53] | 291 |  | 8.7 / 0.0 | 259 | 7.07 | 15 |
| Cf | stock | 1 | 3.67 [2.44-4.89] | 2.84 [2.26-4.00] | 3.35 [2.64-3.52] | 299 |  | 4.7 / 0.1 | 258 | 7.72 | 15 |
| Cf | stock | 2 | 3.88 [2.47-4.43] | 2.97 [2.31-3.84] | 3.42 [2.64-3.47] | 292 |  | 9.7 / 0.1 | 259 | 7.72 | 15 |
| Cf | stock | 4 | 3.82 [3.29-5.01] | 2.92 [2.72-4.12] | 3.46 [3.24-3.52] | 289 |  | 10.9 / 0.2 | 258 | 7.91 | 15 |
| Cf | stock | 8 | 3.80 [2.47-4.86] | 2.88 [2.27-4.04] | 3.42 [2.70-3.56] | 292 |  | 11.3 / 0.1 | 258 | 8.00 | 15 |
| Cf | h2-batch | 1 | 2.26 [2.17-3.37] | 2.00 [1.94-3.13] | 2.58 [2.47-2.68] | 388 |  | 6.1 / 0.0 | 11 | 5.58 | 15 |
| Cf | h2-batch | 2 | 2.29 [2.13-3.40] | 2.01 [1.91-3.16] | 2.49 [2.37-2.79] | 402 |  | 7.2 / 0.0 | 10 | 5.58 | 15 |
| Cf | h2-batch | 4 | 2.13 [2.03-3.37] | 1.94 [1.86-3.22] | 2.65 [2.58-2.88] | 378 |  | 8.1 / 0.1 | 11 | 5.61 | 15 |
| Cf | h2-batch | 8 | 2.14 [2.00-3.34] | 1.95 [1.82-3.14] | 2.64 [2.36-2.75] | 379 |  | 8.0 / 0.1 | 12 | 5.56 | 15 |
| Cf-cb | stock | 1 | 3.45 [2.83-4.31] | 2.50 [2.23-3.42] | 3.41 [3.20-3.62] | 293 |  | 4.2 / 0.1 | 258 | 7.57 | 15 |
| Cf-cb | stock | 2 | 3.52 [2.94-4.49] | 2.60 [2.31-3.54] | 3.36 [3.21-3.63] | 298 |  | 9.9 / 0.1 | 260 | 7.89 | 15 |
| Cf-cb | stock | 4 | 3.55 [3.03-4.59] | 2.63 [2.42-3.62] | 3.38 [3.21-3.52] | 296 |  | 13.4 / 0.0 | 259 | 7.64 | 15 |
| Cf-cb | stock | 8 | 3.52 [2.18-4.49] | 2.64 [2.01-3.57] | 3.41 [2.67-3.52] | 294 |  | 13.4 / 0.0 | 259 | 7.88 | 15 |
| Cf-cb | h2-batch | 1 | 1.87 [1.67-2.89] | 1.59 [1.49-2.59] | 2.40 [2.25-2.57] | 417 |  | 4.9 / 0.0 | 12 | 5.32 | 15 |
| Cf-cb | h2-batch | 2 | 1.72 [1.67-2.73] | 1.56 [1.53-2.56] | 2.61 [2.44-2.82] | 384 |  | 8.5 / 0.0 | 15 | 5.40 | 15 |
| Cf-cb | h2-batch | 4 | 1.86 [1.70-2.75] | 1.62 [1.57-2.62] | 2.54 [2.34-2.70] | 394 |  | 11.2 / 0.0 | 14 | 5.34 | 15 |
| Cf-cb | h2-batch | 8 | 1.96 [1.77-3.10] | 1.68 [1.62-2.86] | 2.43 [2.25-2.72] | 412 |  | 11.2 / 0.0 | 12 | 5.26 | 15 |
| Cf-m4 | stock | 1 | 4.73 [4.13-5.80] | 3.65 [3.32-4.75] | 1.34 [1.27-1.53] | 745 |  | 5.5 / 0.2 | 258 | 7.80 | 15 |
| Cf-m4 | stock | 2 | 5.04 [4.65-6.15] | 3.90 [3.67-5.02] | 1.40 [1.36-1.57] | 712 |  | 11.0 / 0.1 | 259 | 7.76 | 15 |
| Cf-m4 | stock | 4 | 5.17 [4.87-5.91] | 3.94 [3.80-4.84] | 1.43 [1.34-1.64] | 701 |  | 12.1 / 0.2 | 258 | 7.86 | 15 |
| Cf-m4 | stock | 8 | 5.12 [4.44-6.25] | 3.97 [3.60-5.05] | 1.40 [1.34-1.49] | 717 |  | 12.6 / 0.2 | 258 | 7.84 | 15 |
| Cf-m4 | h2-batch | 1 | 2.91 [2.75-3.86] | 2.63 [2.49-3.62] | 1.27 [1.15-1.38] | 790 |  | 6.5 / 0.1 | 13 | 7.08 | 15 |
| Cf-m4 | h2-batch | 2 | 2.96 [2.74-3.96] | 2.65 [2.53-3.70] | 1.27 [1.22-1.37] | 786 |  | 8.5 / 0.1 | 13 | 7.10 | 15 |
| Cf-m4 | h2-batch | 4 | 2.85 [2.72-3.97] | 2.61 [2.50-3.74] | 1.26 [1.21-1.42] | 794 |  | 9.3 / 0.1 | 12 | 6.93 | 15 |
| Cf-m4 | h2-batch | 8 | 2.87 [2.78-4.02] | 2.64 [2.57-3.74] | 1.28 [1.20-1.38] | 779 |  | 10.1 / 0.1 | 13 | 6.96 | 15 |
| Cf-cb-m4 | stock | 1 | 4.11 [3.65-4.99] | 3.11 [2.92-3.99] | 1.30 [1.26-2.03] | 767 |  | 5.5 / 0.0 | 258 | 7.01 | 15 |
| Cf-cb-m4 | stock | 2 | 4.57 [4.12-5.21] | 3.43 [3.24-4.16] | 1.31 [1.24-1.89] | 766 |  | 12.0 / 0.0 | 260 | 7.00 | 15 |
| Cf-cb-m4 | stock | 4 | 4.50 [4.34-5.03] | 3.44 [3.35-4.03] | 1.31 [1.28-1.95] | 763 |  | 15.4 / 0.1 | 259 | 7.20 | 15 |
| Cf-cb-m4 | stock | 8 | 4.35 [4.13-5.19] | 3.38 [3.25-4.20] | 1.29 [1.25-2.00] | 774 |  | 14.7 / 0.1 | 260 | 7.46 | 15 |
| Cf-cb-m4 | h2-batch | 1 | 2.60 [2.38-3.37] | 2.31 [2.13-3.09] | 1.17 [1.08-1.86] | 853 |  | 6.7 / 0.0 | 17 | 5.98 | 15 |
| Cf-cb-m4 | h2-batch | 2 | 2.74 [2.66-3.52] | 2.45 [2.37-3.24] | 1.20 [1.11-1.92] | 835 |  | 10.0 / 0.0 | 18 | 6.43 | 15 |
| Cf-cb-m4 | h2-batch | 4 | 2.76 [2.61-3.48] | 2.49 [2.34-3.17] | 1.19 [1.06-1.86] | 843 |  | 12.1 / 0.0 | 18 | 6.09 | 15 |
| Cf-cb-m4 | h2-batch | 8 | 2.77 [2.72-3.46] | 2.51 [2.44-3.22] | 1.18 [1.10-1.88] | 847 |  | 12.5 / 0.0 | 18 | 6.26 | 15 |
