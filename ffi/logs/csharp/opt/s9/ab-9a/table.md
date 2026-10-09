| payload | dir | arm | mode | after: CPU us/op median [min-max] (per-rep medians); B/op; minflt/op | before: CPU us/op median [min-max] (per-rep medians); B/op; minflt/op |
|---|---|---|---|---|---|
| P1.2 | decode-read | core-ffi | retain | 373.6 [354.3-410.7] (364.2 376.4); 6.691e+05; 0 | 384.2 [370-533.7] (378.4 394.1); 6.66e+05; 0 |
| P2.2 | decode-read | core-ffi | retain | 1832 [1784-2010] (1796 1861); 2.113e+06; 0 | 1956 [1799-2480] (1902 2008); 2.181e+06; 0 |
| P2.2/latin1 | decode-read | core-ffi | retain | 2371 [2349-2450] (2362 2391); 2.113e+06; 0 | 2678 [2421-2905] (2451 2766); 2.181e+06; 0 |
| P2.2/wide | decode-read | core-ffi | retain | 3075 [3010-3133] (3096 3056); 2.113e+06; 0 | 3272 [3124-3585] (3135 3481); 2.181e+06; 0 |
| P2.3 | decode-read | core-ffi | retain | 1433 [1299-1848] (1390 1458); 1.928e+06; 0 | 1316 [1231-1689] (1394 1241); 2.101e+06; 0 |
| P2.4 | decode-read | core-ffi | retain | 1653 [1583-1741] (1675 1619); 2.784e+06; 0 | 1829 [1599-2628] (1621 2161); 3.276e+06; 0 |
| P2.5 | decode-read | core-ffi | retain | 57.11 [54.8-68.85] (56.21 57.48); 7.875e+04; 0 | 59.07 [54.44-91.2] (55.65 61.44); 8.147e+04; 0 |
| P4.1 | decode-read | core-ffi | retain | 294.6 [282.9-338.4] (294.5 298.2); 3.366e+05; 0 | 309.3 [291.2-419] (293.7 321.7); 3.638e+05; 0 |
| U-deep-u-repeated | decode-read | core-ffi | retain | 2.951 [2.833-3.209] (2.869 3.083); 4016; 0 | 3.071 [2.99-3.191] (3.004 3.092); 4152; 0 |
| U-nested-before | decode-read | core-ffi | retain | 0.691 [0.6826-0.7073] (0.691 0.6929); 904; 0 | 0.7004 [0.6866-0.8148] (0.695 0.7038); 904; 0 |
| U-oneof-u-repeated | decode-read | core-ffi | retain | 0.3526 [0.3452-0.3822] (0.3525 0.3533); 400; 0 | 0.3487 [0.3433-0.4147] (0.3459 0.354); 400; 0 |
| U-wire-DualResponse-left-as-wt5 | decode-read | core-ffi | retain | 0.4954 [0.48-0.5157] (0.4872 0.5077); 568; 0 | 0.5083 [0.4896-0.5313] (0.5106 0.5048); 568; 0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | decode-read | core-ffi | retain | 2.674 [2.653-3.06] (2.687 2.669); 2552; 0 | 3.42 [3.294-3.575] (3.316 3.473); 4776; 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | decode-read | core-ffi | retain | 2.841 [2.724-3.689] (2.865 2.841); 3448; 0 | 2.754 [2.699-2.933] (2.712 2.795); 3720; 0 |
