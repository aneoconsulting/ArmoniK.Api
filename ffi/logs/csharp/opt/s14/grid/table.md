# s14 ladder grid: encode-core-hot, core-ffi, process CPU per op (us), median over every round of every rep; per-string steps in ns = step / strings per encode. CONTAINER INSTRUMENTATION.

r0 E0; r1 E0 on the generic transcoder path; r2 E1R with the UTF-16 stub (bytes NOT checked); r3 E1R; r3g E1R without the guard.

R2 copies `len` raw bytes per string: on Latin-1 and wide content its output is SHORTER than the real UTF-8 (2 or 3 bytes per unit), so on those rows R2 also removes output bytes, not only the transcoder.

## DOTNET_TieredPGO=0

| payload | content | mode | strings | R0 | R1 | R2 | R3 | R3g | R3-R0 ns/str | R1-R0 | R2-R1 | R3-R2 | R3-R3g |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| P2.2 | ascii | drop | 17167 | 912.1 | 1003 | 1281 | 1465 | 1459 | 32.18 | 5.298 | 16.17 | 10.72 | 0.330 |
| P2.2 | ascii | retain | 17167 | 980.8 | 1061 | 1614 | 1453 | 1448 | 27.5 | 4.646 | 32.26 | -9.397 | 0.315 |
| P2.2 | latin1 | drop | 17167 | 1148 | 1232 | 1282 | 1536 | 1434 | 22.61 | 4.871 | 2.929 | 14.81 | 5.96 |
| P2.2 | latin1 | retain | 17167 | 1341 | 1472 | 1587 | 1571 | 1470 | 13.39 | 7.599 | 6.731 | -0.942 | 5.877 |
| P2.2 | wide | drop | 17167 | 1460 | 1503 | 1337 | 1708 | 1530 | 14.44 | 2.478 | -9.66 | 21.62 | 10.38 |
| P2.2 | wide | retain | 17167 | 1512 | 1793 | 1446 | 1662 | 1699 | 8.761 | 16.38 | -20.25 | 12.63 | -2.11 |
| P2.4 | ascii | drop | 26267 | 884.5 | 928.4 | 1693 | 2076 | 1831 | 45.37 | 1.672 | 29.12 | 14.58 | 9.324 |
| P2.4 | ascii | retain | 26267 | 867.8 | 990.3 | 1692 | 1923 | 1957 | 40.18 | 4.661 | 26.73 | 8.79 | -1.269 |
| U-deep-u-repeated | corpus | drop | 31 | 1.252 | 1.423 | 2.032 | 2.573 | 2.344 | 42.63 | 5.528 | 19.65 | 17.45 | 7.38 |
| U-deep-u-repeated | corpus | retain | 31 | 2.048 | 2.077 | 2.796 | 3.059 | 2.896 | 32.59 | 0.929 | 23.19 | 8.469 | 5.239 |

## default JIT configuration (every BDN child kept: no slow-mode filter)

| payload | content | mode | strings | R0 | R1 | R2 | R3 | R3g | R3-R0 ns/str | R1-R0 | R2-R1 | R3-R2 | R3-R3g |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| P2.2 | ascii | drop | 17167 | 976.4 | 991.8 | 1103 | 1248 | 1261 | 15.81 | 0.896 | 6.478 | 8.432 | -0.784 |
| P2.2 | ascii | retain | 17167 | 1015 | 1102 | 1370 | 1465 | 1369 | 26.22 | 5.05 | 15.65 | 5.516 | 5.583 |
| P2.2 | latin1 | drop | 17167 | 1136 | 1159 | 1298 | 1337 | 1394 | 11.72 | 1.342 | 8.1 | 2.276 | -3.344 |
| P2.2 | latin1 | retain | 17167 | 1273 | 1380 | 1299 | 1431 | 1333 | 9.228 | 6.248 | -4.747 | 7.727 | 5.738 |
| P2.2 | wide | drop | 17167 | 1581 | 1488 | 1102 | 1404 | 1341 | -10.3 | -5.415 | -22.5 | 17.61 | 3.707 |
| P2.2 | wide | retain | 17167 | 1566 | 1689 | 1340 | 1479 | 1358 | -5.066 | 7.14 | -20.32 | 8.11 | 7.042 |
| P2.4 | ascii | drop | 26267 | 830.6 | 885.6 | 1545 | 1798 | 1702 | 36.83 | 2.092 | 25.09 | 9.651 | 3.649 |
| P2.4 | ascii | retain | 26267 | 844.4 | 927.1 | 1529 | 1964 | 1684 | 42.63 | 3.15 | 22.91 | 16.56 | 10.66 |
| U-deep-u-repeated | corpus | drop | 31 | 1.372 | 1.414 | 2.02 | 2.242 | 2.18 | 28.08 | 1.358 | 19.56 | 7.159 | 2.021 |
| U-deep-u-repeated | corpus | retain | 31 | 1.841 | 1.997 | 2.571 | 2.762 | 2.613 | 29.7 | 5.01 | 18.53 | 6.156 | 4.796 |

