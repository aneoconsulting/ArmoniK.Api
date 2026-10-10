# s15 ladder rows: encode-core-hot, core-ffi, process CPU per op (us), median over every round of every rep (3 BDN host processes per variant); E1R - E0 per string in ns (step / strings per encode). new = D26 code, base = the code before D26 (same session). CONTAINER INSTRUMENTATION.

## DOTNET_TieredPGO=0

| payload | content | mode | new R0 | new R3 | new R3g | base R0 | base R3 | new R3-R0 ns/str | base R3-R0 ns/str | new R3-R3g ns/str |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| P2.2 | ascii | drop | 980.8 | 1357 | 1340 | 872.8 | 1412 | 21.93 | 31.43 | 1.02 |
| P2.2 | ascii | retain | 956.7 | 1478 | 1367 | 979.3 | 1450 | 30.34 | 27.44 | 6.413 |
| P2.2 | latin1 | drop | 1143 | 1356 | 1373 | 1169 | 1583 | 12.38 | 24.12 | -0.992 |
| P2.2 | latin1 | retain | 1360 | 1387 | 1372 | 1377 | 1430 | 1.581 | 3.081 | 0.879 |
| P2.2 | wide | drop | 1364 | 1632 | 1564 | 1413 | 1527 | 15.64 | 6.621 | 3.981 |
| P2.2 | wide | retain | 1507 | 1607 | 1532 | 1527 | 1609 | 5.841 | 4.761 | 4.362 |
| P2.4 | ascii | drop | 841.2 | 1844 | 1869 | 864.3 | 2006 | 38.16 | 43.46 | -0.957 |
| P2.4 | ascii | retain | 892.1 | 1920 | 1830 | 892.2 | 1846 | 39.15 | 36.3 | 3.428 |
| U-deep-u-repeated | corpus | drop | 1.213 | 2.238 | 2.284 | 1.254 | 2.395 | 33.06 | 36.79 | -1.504 |
| U-deep-u-repeated | corpus | retain | 1.863 | 2.723 | 2.851 | 2.184 | 2.886 | 27.75 | 22.65 | -4.126 |

## default JIT configuration (every BDN child kept)

| payload | content | mode | new R0 | new R3 | new R3g | base R0 | base R3 | new R3-R0 ns/str | base R3-R0 ns/str | new R3-R3g ns/str |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| P2.2 | ascii | drop | 902.8 | 1143 | 1169 | 910.7 | 1317 | 14 | 23.66 | -1.507 |
| P2.2 | ascii | retain | 1013 | 1171 | 1170 | 963.8 | 1339 | 9.197 | 21.83 | 0.052 |
| P2.2 | latin1 | drop | 1149 | 1299 | 1353 | 1151 | 1313 | 8.737 | 9.438 | -3.184 |
| P2.2 | latin1 | retain | 1301 | 1254 | 1223 | 1260 | 1425 | -2.751 | 9.586 | 1.806 |
| P2.2 | wide | drop | 1348 | 1619 | 1370 | 1339 | 1457 | 15.79 | 6.88 | 14.52 |
| P2.2 | wide | retain | 1494 | 1403 | 1394 | 1615 | 1495 | -5.317 | -6.97 | 0.526 |
| P2.4 | ascii | drop | 804.5 | 1699 | 1670 | 775.3 | 1773 | 34.05 | 38 | 1.099 |
| P2.4 | ascii | retain | 809.6 | 1636 | 1646 | 825.6 | 1774 | 31.45 | 36.13 | -0.406 |
| U-deep-u-repeated | corpus | drop | 1.21 | 2.22 | 2.061 | 1.225 | 2.15 | 32.58 | 29.82 | 5.147 |
| U-deep-u-repeated | corpus | retain | 1.81 | 2.532 | 2.416 | 1.752 | 2.779 | 23.31 | 33.13 | 3.755 |

