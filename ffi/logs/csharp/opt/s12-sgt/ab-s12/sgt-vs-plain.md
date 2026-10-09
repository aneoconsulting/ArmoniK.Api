# s12: per-rep medians (us), FSM plain vs [SuppressGCTransition]; "below" = the SGT arm's highest rep median under the plain arm's lowest, "above" the reverse, "overlap" otherwise

| row | drop plain (r1, r2) | drop sgt (r1, r2) | drop | no-unknown plain (r1, r2) | no-unknown sgt (r1, r2) | no-unknown |
|---|---|---|---|---|---|---|
| P1.1 | 1.365, 1.413 | 1.439, 1.448 | above | 1.417, 1.396 | 1.454, 1.399 | overlap |
| P1.2 | 354.4, 369.3 | 379.7, 384.1 | above | 412.6, 368 | 378.4, 376 | overlap |
| P1.3 | 19.49, 18.4 | 18.07, 19.38 | overlap | 16.94, 16.95 | 17.24, 17.65 | above |
| P2.1 | 2.925, 3.072 | 3.052, 2.942 | overlap | 2.822, 2.827 | 2.829, 2.69 | overlap |
| P2.2 | 1889, 1888 | 1812, 1762 | below | 1884, 2005 | 1862, 1963 | overlap |
| P2.2/latin1 | 2314, 2347 | 2315, 2498 | overlap | 2453, 2344 | 2367, 2214 | overlap |
| P2.2/wide | 2902, 2960 | 3100, 3081 | above | 3005, 3092 | 3126, 2912 | overlap |
| P2.3 | 1496, 1389 | 1289, 1352 | below | 1449, 1321 | 1162, 1165 | below |
| P2.4 | 1618, 1597 | 1707, 1576 | overlap | 1579, 1573 | 1703, 1619 | above |
| P2.5 | 58.58, 56.7 | 52.03, 56.13 | below | 55.69, 61.09 | 55.83, 54.81 | overlap |
| P3.1 | 28.72, 28.97 | 30.29, 27.89 | overlap | 28.16, 29.46 | 30.47, 27.91 | overlap |
| P4.1 | 311.6, 287.2 | 281.3, 305.1 | overlap | 290.4, 285.3 | 279.2, 278.1 | below |
| P5.1 | 0.2225, 0.2039 | 0.1956, 0.1927 | below | 0.1747, 0.1633 | 0.1532, 0.1533 | below |
| P5.2 | 7.593, 8.075 | 6.07, 6.805 | below | 6.662, 5.389 | 5.604, 5.313 | overlap |
| P5.3 | 615.7, 613.3 | 655.2, 654 | above | 618.1, 684.6 | 646.9, 587.8 | overlap |
| P5.4 | 1431, 1020 | 1109, 1599 | overlap | 1100, 1667 | 1092, 1341 | overlap |
| P6.1 | 249.8, 243.4 | 223.5, 226.7 | below | 244.7, 235 | 224.4, 220.8 | below |
| P7.1 | 0.6452, 0.6194 | 0.5383, 0.5423 | below | 0.5651, 0.563 | 0.5017, 0.484 | below |
| U-deep-u-repeated | 2.878, 2.875 | 2.762, 2.855 | below | 2.718, 2.639 | 2.571, 2.492 | below |
| U-nested-before | 0.5281, 0.5085 | 0.5154, 0.4834 | overlap | 0.5281, 0.483 | 0.4612, 0.5077 | overlap |
| U-oneof-u-repeated | 0.2669, 0.2676 | 0.2387, 0.2432 | below | 0.2316, 0.2364 | 0.2168, 0.2035 | below |
| U-wire-DualResponse-left-as-wt5 | 0.4207, 0.4226 | 0.4039, 0.3833 | below | 0.3885, 0.3752 | 0.3588, 0.3552 | below |
| U-wire-ListMetricsResponse-batches-as-wt0 | 2.576, 2.546 | 2.528, 2.322 | below | 2.522, 2.56 | 2.41, 2.224 | below |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | 2.812, 2.635 | 2.69, 2.542 | overlap | 2.619, 2.595 | 2.543, 2.576 | below |
| U-wire-UploadResultDataMessage-upload-as-wt5 | 0.2016, 0.1938 | 0.1878, 0.1851 | below | 0.1634, 0.1686 | 0.1619, 0.1518 | below |
