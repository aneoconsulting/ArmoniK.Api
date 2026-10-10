# s14 ladder grid: encode-core-hot, core-ffi, process CPU per op (us), median over every round of every rep; per-string steps in ns = step / strings per encode. CONTAINER INSTRUMENTATION.

r0 E0; r1 E0 on the generic transcoder path; r2 E1R with the UTF-16 stub (bytes NOT checked); r3 E1R; r3g E1R without the guard.

R2 copies `len` raw bytes per string: on Latin-1 and wide content its output is SHORTER than the real UTF-8 (2 or 3 bytes per unit), so on those rows R2 also removes output bytes, not only the transcoder.

## DOTNET_TieredPGO=0

| payload | content | mode | strings | R0 | R1 | R2 | R3 | R3g | R3-R0 ns/str | R1-R0 | R2-R1 | R3-R2 | R3-R3g |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| P2.2 | ascii | drop | 17167 | 920.6 | 1020 | 1238 | 1436 | 1430 | 30.02 | 5.762 | 12.74 | 11.52 | 0.345 |
| P2.2 | ascii | retain | 17167 | 1006 | 1107 | 1434 | 1489 | 1413 | 28.14 | 5.888 | 19.01 | 3.241 | 4.438 |
| P2.2 | latin1 | drop | 17167 | 1170 | 1256 | 1271 | 1490 | 1427 | 18.63 | 5.03 | 0.868 | 12.73 | 3.643 |
| P2.2 | latin1 | retain | 17167 | 1356 | 1462 | 1535 | 1565 | 1472 | 12.13 | 6.138 | 4.278 | 1.718 | 5.403 |
| P2.2 | wide | drop | 17167 | 1353 | 1503 | 1255 | 1600 | 1549 | 14.42 | 8.743 | -14.42 | 20.1 | 2.989 |
| P2.2 | wide | retain | 17167 | 1537 | 1707 | 1496 | 1690 | 1609 | 8.96 | 9.943 | -12.28 | 11.3 | 4.724 |
| P2.4 | ascii | drop | 26267 | 854 | 939.7 | 1717 | 1961 | 1835 | 42.15 | 3.263 | 29.59 | 9.294 | 4.801 |
| P2.4 | ascii | retain | 26267 | 889.9 | 986.7 | 1685 | 1986 | 1923 | 41.71 | 3.685 | 26.59 | 11.44 | 2.368 |
| U-deep-u-repeated | corpus | drop | 31 | 1.271 | 1.427 | 2.057 | 2.52 | 2.334 | 40.28 | 5.021 | 20.32 | 14.94 | 5.992 |
| U-deep-u-repeated | corpus | retain | 31 | 1.936 | 2.062 | 2.84 | 2.963 | 2.892 | 33.15 | 4.09 | 25.08 | 3.975 | 2.289 |

## default JIT configuration (every BDN child kept: no slow-mode filter)

| payload | content | mode | strings | R0 | R1 | R2 | R3 | R3g | R3-R0 ns/str | R1-R0 | R2-R1 | R3-R2 | R3-R3g |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| P2.2 | ascii | drop | 17167 | 919.3 | 1061 | 1124 | 1258 | 1262 | 19.74 | 8.238 | 3.669 | 7.838 | -0.200 |
| P2.2 | ascii | retain | 17167 | 1014 | 1091 | 1370 | 1439 | 1289 | 24.75 | 4.476 | 16.22 | 4.05 | 8.755 |
| P2.2 | latin1 | drop | 17167 | 1086 | 1199 | 1146 | 1356 | 1240 | 15.73 | 6.588 | -3.08 | 12.22 | 6.739 |
| P2.2 | latin1 | retain | 17167 | 1273 | 1413 | 1350 | 1384 | 1340 | 6.493 | 8.188 | -3.669 | 1.974 | 2.561 |
| P2.2 | wide | drop | 17167 | 1315 | 1448 | 1099 | 1440 | 1333 | 7.291 | 7.722 | -20.33 | 19.9 | 6.267 |
| P2.2 | wide | retain | 17167 | 1549 | 1687 | 1339 | 1482 | 1385 | -3.856 | 8.09 | -20.3 | 8.354 | 5.682 |
| P2.4 | ascii | drop | 26267 | 810.1 | 886.3 | 1499 | 1765 | 1695 | 36.35 | 2.902 | 23.34 | 10.11 | 2.638 |
| P2.4 | ascii | retain | 26267 | 821.3 | 926.3 | 1529 | 1775 | 1693 | 36.33 | 3.999 | 22.94 | 9.384 | 3.129 |
| U-deep-u-repeated | corpus | drop | 31 | 1.246 | 1.406 | 1.803 | 2.212 | 2.16 | 31.16 | 5.154 | 12.8 | 13.21 | 1.689 |
| U-deep-u-repeated | corpus | retain | 31 | 1.859 | 2.001 | 2.648 | 2.771 | 2.636 | 29.42 | 4.584 | 20.84 | 3.994 | 4.355 |

