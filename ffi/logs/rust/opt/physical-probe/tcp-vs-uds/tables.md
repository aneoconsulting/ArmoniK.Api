# Rust host: UDS against TCP loopback (2026-10-01)

Stack binary (p1+p2+p3+p5+p6+p7+p8+p9, knobs off except AK_SPARES=6 AK_SPARE_LOCK=1), crates.io h2, pinned allocator, host and core 8 workers, one server (8 workers, 5-8,15-18) on the pinned Unix socket and on TCP 127.0.0.1 (pinned configuration, TCP_NODELAY on accept), pinned client configuration. gen/tcp_vs_uds.sh; conditions uds and tcp alternated, 3 processes each, affinity checked around every process (30 of 30 jsonl carry the affinity line).

TCP_NODELAY on the live client sockets of the timed TCP processes (getsockopt, AK_EXPECT_NODELAY=1 would have aborted otherwise):
          6 # tcp sockets after the timed rounds: 5, TCP_NODELAY on: 5
          9 # tcp sockets after the timed rounds: 6, TCP_NODELAY on: 6
          6 # tcp sockets after the warm-up: 5, TCP_NODELAY on: 5 (getsockopt on the live sockets)
          9 # tcp sockets after the warm-up: 6, TCP_NODELAY on: 6 (getsockopt on the live sockets)

## 1. Timed, in-process (timed/tables.md): client process CPU and wall per call, median [p10-p90], per-process gap to A, voluntary switches / minor faults

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

## 2. Writes per call on the writing thread (untimed /proc run, attr/)

- tcp 16MiB k=1: A 1035 x 16 KiB, Df 1047 x 16 KiB, Df-1f 1035 x 16 KiB, Cf 1036 x 16 KiB, Cf-cb 1046 x 16 KiB, Cf-zc 1034 x 16 KiB
- tcp 16MiB k=8: A 1035 x 16 KiB, Df 1031 x 16 KiB, Df-1f 1031 x 16 KiB, Cf 1035 x 16 KiB, Cf-cb 1036 x 16 KiB, Cf-zc 1033 x 16 KiB
- tcp 4MiB k=1: A 262 x 16 KiB, Df 264 x 16 KiB, Df-1f 261 x 16 KiB, Cf 261 x 16 KiB, Cf-cb 264 x 16 KiB, Cf-zc 259 x 16 KiB
- tcp P5.4 k=1: A 259 x 16 KiB, Df 258 x 16 KiB, Df-1f 258 x 16 KiB, Cf 259 x 16 KiB, Cf-cb 258 x 16 KiB
- tcp P5.4 k=8: A 259 x 16 KiB, Df 259 x 16 KiB, Df-1f 259 x 16 KiB, Cf 257 x 16 KiB, Cf-cb 259 x 16 KiB
- uds 16MiB k=1: A 1046 x 16 KiB, Df 1047 x 16 KiB, Df-1f 1045 x 16 KiB, Cf 1047 x 16 KiB, Cf-cb 1048 x 16 KiB, Cf-zc 1042 x 16 KiB
- uds 16MiB k=8: A 1051 x 16 KiB, Df 1051 x 16 KiB, Df-1f 1049 x 16 KiB, Cf 1047 x 16 KiB, Cf-cb 1050 x 16 KiB, Cf-zc 1053 x 16 KiB
- uds 4MiB k=1: A 264 x 16 KiB, Df 263 x 16 KiB, Df-1f 264 x 16 KiB, Cf 265 x 15 KiB, Cf-cb 266 x 15 KiB, Cf-zc 263 x 16 KiB
- uds P5.4 k=1: A 265 x 15 KiB, Df 265 x 15 KiB, Df-1f 264 x 16 KiB, Cf 266 x 15 KiB, Cf-cb 262 x 16 KiB
- uds P5.4 k=8: A 263 x 16 KiB, Df 261 x 16 KiB, Df-1f 263 x 16 KiB, Cf 263 x 16 KiB, Cf-cb 262 x 16 KiB

## 3. Server CPU per call (perf stat -p <server>, counting one cell's timed rounds; server-cpu/): task-clock ms / context switches / M cycles

| workload | cell | UDS | TCP |
|---|---|---|---|
| 16MiB k=1 | A | 9.48 / 199 / 30.2 | 11.25 / 390 / 35.0 |
| 16MiB k=1 | Df | 10.66 / 176 / 34.2 | 11.21 / 366 / 35.0 |
| 16MiB k=1 | Df-1f | 10.25 / 228 / 32.4 | 11.21 / 365 / 35.0 |
| 16MiB k=1 | Cf | 11.31 / 342 / 35.1 | 10.86 / 349 / 33.9 |
| 16MiB k=1 | Cf-cb | 11.70 / 380 / 36.2 | 10.93 / 354 / 34.1 |
| 16MiB k=1 | Cf-zc | 11.10 / 342 / 34.8 | 10.82 / 373 / 33.7 |
| 16MiB k=8 | A | 21.20 / 1061 / 63.8 | 23.00 / 1250 / 68.7 |
| 16MiB k=8 | Df | 22.79 / 1064 / 69.0 | 15.38 / 347 / 48.7 |
| 16MiB k=8 | Df-1f | 20.45 / 1105 / 61.2 | 15.07 / 399 / 47.3 |
| 16MiB k=8 | Cf | 20.44 / 1059 / 61.3 | 23.77 / 1218 / 71.3 |
| 16MiB k=8 | Cf-cb | 20.31 / 1045 / 60.8 | 15.92 / 378 / 50.2 |
| 16MiB k=8 | Cf-zc | 19.73 / 1027 / 59.1 | 24.15 / 1271 / 72.2 |
| P5.4 k=1 | A | 3.27 / 82 / 10.2 | 2.96 / 90 / 9.3 |
| P5.4 k=1 | Df | 2.74 / 57 / 8.7 | 2.95 / 88 / 9.2 |
| P5.4 k=1 | Df-1f | 2.89 / 76 / 9.0 | 3.00 / 100 / 9.3 |
| P5.4 k=1 | Cf | 2.55 / 30 / 8.2 | 2.97 / 91 / 9.3 |
| P5.4 k=1 | Cf-cb | 2.93 / 79 / 9.1 | 2.93 / 110 / 9.1 |

One process per (transport, workload, cell), 18 calls (k=1) or 32 (k=8): at k=8 the TCP rows split into two groups (about 15 and 23 ms) with no pattern by cell; read them as one sample each.

## 4. Where the client CPUs' cycles go on TCP (accounting/: one-cell processes, d/16 k=1, perf stat and perf record with the booted System.map)

```
uds A softirq ticks on the client CPUs over the whole process (warm-up included): 0
uds Cf softirq ticks on the client CPUs over the whole process (warm-up included): 0
tcp A softirq ticks on the client CPUs over the whole process (warm-up included): 18
tcp Cf softirq ticks on the client CPUs over the whole process (warm-up included): 18
```

| transport | cell | process CPU per call (CLOCK_PROCESS_CPUTIME_ID) | perf task-clock | cycles user / kernel (M) | sampled M cycles per call by bucket |
|---|---|---|---|---|---|
| uds | A | 8.04 ms | 8.12 ms | 8.8 / 17.4 | total 25.4: socket write: copy from user 4.7, socket write: skb alloc/free, memcg 3.8, socket write: other 2.9, socket write: zeroing new skb pages 2.7, scheduling, interrupts 1.1, epoll_wait 0.7; leaf in loadable modules 0.0 |
| uds | Cf | 8.57 ms | 8.68 ms | 9.1 / 18.7 | total 25.1: socket write: copy from user 7.7, socket write: other 3.2, socket write: skb alloc/free, memcg 2.6, socket write: zeroing new skb pages 2.2, socket write: wake the reader 0.8, scheduling, interrupts 0.8; leaf in loadable modules 0.0 |
| tcp | A | 10.09 ms | 15.47 ms | 8.6 / 42.0 | total 50.5: loopback receive path 19.9, socket write: other 12.3, socket write: copy from user 3.9, socket write: zeroing new skb pages 3.4, socket write: skb alloc/free, memcg 1.6, scheduling, interrupts 1.0; leaf in loadable modules 14.3 |
| tcp | Cf | 10.69 ms | 15.60 ms | 9.2 / 41.7 | total 48.8: loopback receive path 18.0, socket write: other 13.6, socket write: copy from user 4.4, socket write: zeroing new skb pages 3.5, socket write: skb alloc/free, memcg 1.8, scheduling, interrupts 0.6; leaf in loadable modules 13.7 |

Kernel: CONFIG_IRQ_TIME_ACCOUNTING=y and CONFIG_VIRT_CPU_ACCOUNTING_GEN=y (/proc/config.gz). Loadable netfilter modules present (nf_tables, nft_compat, nf_conntrack, nf_nat, br_netfilter, xt_* , ipt_REJECT; /proc/modules). Leaf addresses beyond the kernel image's _etext are counted as [module] (their symbols are not in System.map). The first perf record pass (record/) was classified before the [module] rule and resolved those addresses to handshake_exit; its bucket split is otherwise the same.
