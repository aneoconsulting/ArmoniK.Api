# in-process comparison: commit dd7384bc + UNCOMMITTED; 2026-10-01T19:22:03Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# isolation  cmdline: isolated='' nohz_full=''
# cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x10, 0,9-10,19 x39, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 1.46 2.12 1.84 2/934 897789
# affinity checks: every server thread allowed exactly AK_CPU_SERVER, checked before and after every client process; every client thread allowed exactly AK_CPU_CLIENT, checked by the probe before and after its timed rounds (AK_EXPECT_CPUS); a mismatch aborts
# transport tcp (client target http://127.0.0.1:33275, TCP_NODELAY read back on every client socket); base env AK_PROBE_TASKCLOCK=1 AK_PROBE_SERVER_PID=897467 AK_RPC_TARGET=http://127.0.0.1:33275 AK_EXPECT_NODELAY=1
# server also on TCP 127.0.0.1:33275 (pinned configuration, TCP_NODELAY on accept)
# netfilter (read-only): modules nft_log nft_limit xt_limit xt_NFLOG nfnetlink_log xt_physdev xt_multiport nf_conntrack_netlink xt_mark xt_nfacct nfnetlink_acct xt_comment xt_set ip_set xt_addrtype xt_CHECKSUM xt_MASQUERADE xt_conntrack ipt_REJECT nf_reject_ipv4 xt_tcpudp nft_compat nft_chain_nat nf_tables nfnetlink ip6_udp_tunnel nfit xt_nat br_netfilter nf_nat bridge nf_conntrack nf_defrag_ipv6 nf_defrag_ipv4 
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 897467, pinned socket
# cells A,Cf (direction c: A,Cf) (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2; d16k16 5 x 1 batch of 16 warm 1; d16k32 4 x 1 x 32 warm 1; c54k16 6 x 2 x 16 warm 1; c54k32 5 x 2 x 32 warm 1); repetitions 1
# condition stock: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/stock/release/libak_core.so (sha256 fe9f27d51c2a95ba), env LD_LIBRARY_PATH=/data/csdt/ak-cores/stock/release,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h2-batch: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e23f6808b69bc882), core /data/csdt/ak-cores/h2-batch/release/libak_core.so (sha256 6e5a84caeab66f39), env LD_LIBRARY_PATH=/data/csdt/ak-cores/h2-batch/release,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 12 s
# at the end: cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; smt on (active 1); no_turbo 1; governor cpu1 performance; scaling min/max cpu1 3300000/3300000 kHz
# at the end: isolation  cmdline: isolated='' nohz_full=''
# at the end: cgroups    cpuset.cpus.effective: init.scope=0,9-10,19 system.slice=0,9-10,19 user.slice=0-19 machine.slice=0,9-10,19; this driver's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope: n/a; root cpuset.cpus.isolated=''; this driver's affinity 0,9,10,19
# at the end: irq        default_smp_affinity fffff; smp_affinity_list of /proc/irq/*: 0-1,10 x1, 0-19 x10, 0,9-10,19 x39, 11,13,15 x1, 17,19 x1, 2-3,12 x1, 4-5,14 x1, 6-7,16 x1, 8-9,18 x1, 
# at the end: allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: WaylandEventThr x16, QDBusConnection x9, .drkonqi-coredu x8, sddm-helper x1, systemd x1, .kwin_wayland-w x1, HDMI-A-1 x1, DP-1 x1, libinput-connec x1, QQmlThread x1, fusermount3 x1
# at the end: running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: ; loadavg 1.89 2.18 1.86 1/933 898333

## d16k1

| cell | stock: CPU ms | stock: wall ms | stock: gap to A per rep (range) | stock: vcs / ics / minflt | h2-batch: CPU ms | h2-batch: wall ms | h2-batch: gap to A per rep (range) | h2-batch: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 9.94 [9.86-10.09] | 14.54 [14.39-14.66] |  | 37.1 / 0.4 / 0.0 | 10.05 [9.99-10.13] | 14.63 [14.53-14.68] |  | 37.0 / 0.4 / 0.0 |
| Cf | 10.54 [10.43-11.13] | 13.33 [13.17-13.52] | +0.61 (0.00) | 44.6 / 0.2 / 0.2 | 6.87 [6.78-6.97] | 10.50 [9.56-11.17] | -3.18 (0.00) | 23.9 / 0.1 / 0.2 |

## d16k8

| cell | stock: CPU ms | stock: wall ms | stock: gap to A per rep (range) | stock: vcs / ics / minflt | h2-batch: CPU ms | h2-batch: wall ms | h2-batch: gap to A per rep (range) | h2-batch: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 11.25 [10.72-12.16] | 13.19 [12.91-13.29] |  | 34.8 / 0.2 / 240.2 | 11.34 [11.09-12.15] | 13.16 [12.95-13.53] |  | 34.6 / 0.2 / 224.2 |
| Cf | 10.94 [10.76-11.64] | 12.75 [12.52-13.14] | -0.31 (0.00) | 37.3 / 0.2 / 0.4 | 7.50 [7.19-7.67] | 9.50 [8.23-9.96] | -3.84 (0.00) | 30.3 / 0.1 / 0.5 |
