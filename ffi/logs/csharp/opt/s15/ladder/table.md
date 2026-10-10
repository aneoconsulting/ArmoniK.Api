# s15 ladder rows: encode-core-hot, core-ffi, process CPU per op (us), median over every round of every rep (3 BDN host processes per variant); E1R - E0 per string in ns (step / strings per encode). new = D26 code, base = the code before D26 (same session). CONTAINER INSTRUMENTATION.

## DOTNET_TieredPGO=0

| payload | content | mode | new R0 | new R3 | new R3g | base R0 | base R3 | new R3-R0 ns/str | base R3-R0 ns/str | new R3-R3g ns/str |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| P2.2 | ascii | drop | 897.9 | 1331 | 1350 | 885.3 | 1436 | 25.22 | 32.07 | -1.135 |
| P2.2 | ascii | retain | 1024 | 1443 | 1375 | 994.5 | 1472 | 24.43 | 27.84 | 3.992 |
| P2.2 | latin1 | drop | 1160 | 1394 | 1391 | 1158 | 1544 | 13.67 | 22.48 | 0.180 |
| P2.2 | latin1 | retain | 1343 | 1419 | 1388 | 1337 | 1520 | 4.428 | 10.64 | 1.834 |
| P2.2 | wide | drop | 1430 | 1551 | 1561 | 1381 | 1597 | 7.043 | 12.61 | -0.566 |
| P2.2 | wide | retain | 1522 | 1585 | 1552 | 1532 | 1610 | 3.673 | 4.529 | 1.9 |
| P2.4 | ascii | drop | 861.8 | 1856 | 1927 | 883.7 | 1994 | 37.83 | 42.26 | -2.703 |
| P2.4 | ascii | retain | 903 | 1912 | 1881 | 898.2 | 1881 | 38.4 | 37.42 | 1.158 |
| U-deep-u-repeated | corpus | drop | 1.249 | 2.28 | 2.266 | 1.271 | 2.438 | 33.26 | 37.64 | 0.445 |
| U-deep-u-repeated | corpus | retain | 1.835 | 2.8 | 2.83 | 1.961 | 2.898 | 31.14 | 30.22 | -0.950 |

## default JIT configuration (every BDN child kept)

| payload | content | mode | new R0 | new R3 | new R3g | base R0 | base R3 | new R3-R0 ns/str | base R3-R0 ns/str | new R3-R3g ns/str |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| P2.2 | ascii | drop | 900.6 | 1196 | 1169 | 915.5 | 1284 | 17.21 | 21.44 | 1.576 |
| P2.2 | ascii | retain | 998.3 | 1216 | 1277 | 1020 | 1333 | 12.66 | 18.25 | -3.567 |
| P2.2 | latin1 | drop | 1146 | 1294 | 1294 | 1147 | 1333 | 8.624 | 10.82 | -0.005 |
| P2.2 | latin1 | retain | 1292 | 1211 | 1233 | 1314 | 1405 | -4.754 | 5.324 | -1.289 |
| P2.2 | wide | drop | 1308 | 1326 | 1370 | 1351 | 1435 | 1.056 | 4.869 | -2.582 |
| P2.2 | wide | retain | 1559 | 1388 | 1360 | 1543 | 1556 | -9.967 | 0.752 | 1.622 |
| P2.4 | ascii | drop | 789 | 1694 | 1686 | 784.7 | 1779 | 34.44 | 37.84 | 0.278 |
| P2.4 | ascii | retain | 809.6 | 1682 | 1689 | 813.1 | 1758 | 33.2 | 35.95 | -0.284 |
| U-deep-u-repeated | corpus | drop | 1.212 | 2.095 | 2.041 | 1.208 | 2.207 | 28.47 | 32.23 | 1.742 |
| U-deep-u-repeated | corpus | retain | 1.802 | 2.68 | 2.509 | 1.77 | 2.77 | 28.3 | 32.26 | 5.489 |

