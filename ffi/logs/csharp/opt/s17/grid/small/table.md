# s17 small set: core-ffi encode rows, eager (today) vs lazy nested render, process CPU per op, median over every round of every rep (8 reps = 8 BDN host processes per variant, interleaved). Lazy - eager: the difference of those medians, and [min, max] of the per-rep differences. ns, or us where marked. CONTAINER INSTRUMENTATION.

## E0

| payload | content | dir | mode | eager | lazy | lazy - eager | per-rep diff [min, max] | unit |
|---|---|---|---|---:|---:|---:|---|---|
| P1.1 | ascii | encode-core-hot | drop | 713.6 | 717.1 | 3.448 | [-62.63, 63.94] | ns |
| P1.1 | ascii | encode-core-hot | retain | 1034 | 727.8 | -306.6 | [-534.1, -285.6] | ns |
| P1.1 | ascii | encode-transport | drop | 747.7 | 811.3 | 63.54 | [38.22, 185.2] | ns |
| P1.1 | ascii | encode-transport | retain | 1137 | 812.9 | -324.5 | [-407.7, -301.3] | ns |
| P5.1 | ascii | encode-core-hot | drop | 101.8 | 99.85 | -1.989 | [-10.24, 4.293] | ns |
| P5.1 | ascii | encode-core-hot | retain | 101.9 | 103.4 | 1.492 | [-18.9, 9.581] | ns |
| P5.1 | ascii | encode-transport | drop | 135 | 132.2 | -2.848 | [-17.32, 9.485] | ns |
| P5.1 | ascii | encode-transport | retain | 137.2 | 138.8 | 1.518 | [-6.578, 9.222] | ns |
| P7.1 | ascii | encode-core-hot | drop | 259.8 | 252.4 | -7.353 | [-16.61, 10.15] | ns |
| P7.1 | ascii | encode-core-hot | retain | 276.4 | 274.4 | -2.004 | [-23.64, 39.93] | ns |
| P7.1 | ascii | encode-transport | drop | 309.5 | 299.7 | -9.796 | [-26.05, 27.69] | ns |
| P7.1 | ascii | encode-transport | retain | 321.9 | 317.6 | -4.289 | [-27.88, 16.82] | ns |
| U-deep-u-repeated | corpus | encode-core-hot | drop | 1183 | 1089 | -94.65 | [-355.2, 79.17] | ns |
| U-deep-u-repeated | corpus | encode-core-hot | retain | 1708 | 1150 | -558.2 | [-794.1, -452.7] | ns |
| U-deep-u-repeated | corpus | encode-transport | drop | 1199 | 1169 | -29.64 | [-136.6, 91.2] | ns |
| U-deep-u-repeated | corpus | encode-transport | retain | 1829 | 1214 | -614.6 | [-823.2, -431.9] | ns |
| U-nested-before | corpus | encode-core-hot | drop | 219.5 | 207.1 | -12.34 | [-27, 17.77] | ns |
| U-nested-before | corpus | encode-core-hot | retain | 551 | 229.7 | -321.3 | [-345.7, -297.4] | ns |
| U-nested-before | corpus | encode-transport | drop | 244.3 | 252.8 | 8.496 | [-12.1, 49.41] | ns |
| U-nested-before | corpus | encode-transport | retain | 597.9 | 285.3 | -312.7 | [-352.1, -279.3] | ns |
| U-oneof-u-repeated | corpus | encode-core-hot | drop | 111.2 | 112.6 | 1.406 | [-32.99, 13.49] | ns |
| U-oneof-u-repeated | corpus | encode-core-hot | retain | 125 | 123.3 | -1.73 | [-10.01, 9.918] | ns |
| U-oneof-u-repeated | corpus | encode-transport | drop | 145 | 151.1 | 6.058 | [-5.023, 19] | ns |
| U-oneof-u-repeated | corpus | encode-transport | retain | 159.8 | 160.8 | 0.985 | [-17.46, 8.026] | ns |
| U-wire-DualResponse-left-as-wt5 | corpus | encode-core-hot | drop | 197.3 | 202.5 | 5.213 | [-10.26, 33.51] | ns |
| U-wire-DualResponse-left-as-wt5 | corpus | encode-core-hot | retain | 223.3 | 210.8 | -12.45 | [-24.56, 9.346] | ns |
| U-wire-DualResponse-left-as-wt5 | corpus | encode-transport | drop | 235 | 229.8 | -5.282 | [-23.67, 18.19] | ns |
| U-wire-DualResponse-left-as-wt5 | corpus | encode-transport | retain | 253.8 | 249.5 | -4.227 | [-43.76, 16.78] | ns |
| U-wire-ListMetricsResponse-batches-as-wt0 | corpus | encode-core-hot | drop | 1088 | 1136 | 47.93 | [-10.75, 133] | ns |
| U-wire-ListMetricsResponse-batches-as-wt0 | corpus | encode-core-hot | retain | 1092 | 1111 | 18.48 | [-44.49, 204.6] | ns |
| U-wire-ListMetricsResponse-batches-as-wt0 | corpus | encode-transport | drop | 1142 | 1151 | 9.396 | [-62.4, 103.3] | ns |
| U-wire-ListMetricsResponse-batches-as-wt0 | corpus | encode-transport | retain | 1214 | 1187 | -26.89 | [-129, 92.42] | ns |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | corpus | encode-core-hot | drop | 1092 | 1112 | 20.59 | [-71.07, 93.74] | ns |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | corpus | encode-core-hot | retain | 1094 | 1121 | 26.84 | [-13.77, 152.5] | ns |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | corpus | encode-transport | drop | 1135 | 1153 | 17.39 | [-49.42, 159] | ns |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | corpus | encode-transport | retain | 1181 | 1180 | -0.873 | [-58.14, 118.5] | ns |
| U-wire-UploadResultDataMessage-upload-as-wt5 | corpus | encode-core-hot | drop | 100.7 | 99.87 | -0.785 | [-5.464, 2.236] | ns |
| U-wire-UploadResultDataMessage-upload-as-wt5 | corpus | encode-core-hot | retain | 110.6 | 108.9 | -1.725 | [-34.21, 6.812] | ns |
| U-wire-UploadResultDataMessage-upload-as-wt5 | corpus | encode-transport | drop | 132.5 | 132.7 | 0.211 | [-28.7, 23.1] | ns |
| U-wire-UploadResultDataMessage-upload-as-wt5 | corpus | encode-transport | retain | 135 | 137.9 | 2.853 | [-6.246, 12.97] | ns |

## E1R:128

| payload | content | dir | mode | eager | lazy | lazy - eager | per-rep diff [min, max] | unit |
|---|---|---|---|---:|---:|---:|---|---|
| P1.1 | ascii | encode-core-hot | drop | 708.7 | 694.5 | -14.25 | [-72.89, 28.17] | ns |
| P1.1 | ascii | encode-core-hot | retain | 1044 | 739.6 | -304.5 | [-371.8, -220.6] | ns |
| P1.1 | ascii | encode-transport | drop | 750.6 | 829.3 | 78.71 | [46.52, 102.5] | ns |
| P1.1 | ascii | encode-transport | retain | 1142 | 838.9 | -302.7 | [-369, -241.6] | ns |
| P5.1 | ascii | encode-core-hot | drop | 100.7 | 100.7 | -0.043 | [-7.686, 3.791] | ns |
| P5.1 | ascii | encode-core-hot | retain | 100.9 | 104.3 | 3.35 | [-5.054, 11.86] | ns |
| P5.1 | ascii | encode-transport | drop | 134.7 | 137.7 | 2.951 | [-2.499, 8.011] | ns |
| P5.1 | ascii | encode-transport | retain | 138.4 | 139.7 | 1.295 | [-11.41, 31.64] | ns |
| P7.1 | ascii | encode-core-hot | drop | 257.3 | 251.7 | -5.564 | [-15.35, 19.44] | ns |
| P7.1 | ascii | encode-core-hot | retain | 276.7 | 261 | -15.74 | [-52.75, 10.55] | ns |
| P7.1 | ascii | encode-transport | drop | 317.1 | 303.4 | -13.67 | [-34.48, 0.496] | ns |
| P7.1 | ascii | encode-transport | retain | 326 | 319.8 | -6.228 | [-27.12, 7.705] | ns |
| U-deep-u-repeated | corpus | encode-core-hot | drop | 1168 | 1109 | -59.41 | [-187.6, -20.08] | ns |
| U-deep-u-repeated | corpus | encode-core-hot | retain | 1693 | 1185 | -507.9 | [-574.6, -423.9] | ns |
| U-deep-u-repeated | corpus | encode-transport | drop | 1233 | 1201 | -31.59 | [-115.8, 66.08] | ns |
| U-deep-u-repeated | corpus | encode-transport | retain | 1806 | 1202 | -604.5 | [-696.9, -517.6] | ns |
| U-nested-before | corpus | encode-core-hot | drop | 212.8 | 213.4 | 0.592 | [-13.34, 13.81] | ns |
| U-nested-before | corpus | encode-core-hot | retain | 538.9 | 224.7 | -314.2 | [-360.4, -293.6] | ns |
| U-nested-before | corpus | encode-transport | drop | 249.5 | 254.6 | 5.106 | [-19.82, 30.09] | ns |
| U-nested-before | corpus | encode-transport | retain | 607.2 | 278.1 | -329.1 | [-402.1, -299.3] | ns |
| U-oneof-u-repeated | corpus | encode-core-hot | drop | 111.4 | 111.3 | -0.137 | [-5.034, 16.87] | ns |
| U-oneof-u-repeated | corpus | encode-core-hot | retain | 119.6 | 121.6 | 1.967 | [-8.968, 8.843] | ns |
| U-oneof-u-repeated | corpus | encode-transport | drop | 148.4 | 151.8 | 3.442 | [-7.999, 7.726] | ns |
| U-oneof-u-repeated | corpus | encode-transport | retain | 162 | 160.8 | -1.166 | [-8.485, 23.56] | ns |
| U-wire-DualResponse-left-as-wt5 | corpus | encode-core-hot | drop | 199.7 | 190.8 | -8.894 | [-18.15, 5.375] | ns |
| U-wire-DualResponse-left-as-wt5 | corpus | encode-core-hot | retain | 215.1 | 207.6 | -7.528 | [-16.15, 12.73] | ns |
| U-wire-DualResponse-left-as-wt5 | corpus | encode-transport | drop | 237.8 | 231.5 | -6.266 | [-41.11, 5.26] | ns |
| U-wire-DualResponse-left-as-wt5 | corpus | encode-transport | retain | 261.4 | 251.6 | -9.822 | [-36.64, 3.125] | ns |
| U-wire-ListMetricsResponse-batches-as-wt0 | corpus | encode-core-hot | drop | 1086 | 1098 | 12.2 | [-67.75, 218.8] | ns |
| U-wire-ListMetricsResponse-batches-as-wt0 | corpus | encode-core-hot | retain | 1121 | 1099 | -22.75 | [-123.8, 56.12] | ns |
| U-wire-ListMetricsResponse-batches-as-wt0 | corpus | encode-transport | drop | 1193 | 1147 | -46.54 | [-226.2, 100.5] | ns |
| U-wire-ListMetricsResponse-batches-as-wt0 | corpus | encode-transport | retain | 1218 | 1187 | -31.05 | [-129.9, 83.67] | ns |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | corpus | encode-core-hot | drop | 1084 | 1112 | 27.54 | [-71.27, 264.2] | ns |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | corpus | encode-core-hot | retain | 1110 | 1107 | -3.308 | [-91.46, 69.36] | ns |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | corpus | encode-transport | drop | 1117 | 1134 | 16.8 | [-46.88, 74.32] | ns |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | corpus | encode-transport | retain | 1163 | 1187 | 23.4 | [-54.83, 107.7] | ns |
| U-wire-UploadResultDataMessage-upload-as-wt5 | corpus | encode-core-hot | drop | 100 | 101.2 | 1.189 | [-7.242, 23.48] | ns |
| U-wire-UploadResultDataMessage-upload-as-wt5 | corpus | encode-core-hot | retain | 110.2 | 108.1 | -2.071 | [-14.72, 3.984] | ns |
| U-wire-UploadResultDataMessage-upload-as-wt5 | corpus | encode-transport | drop | 129.5 | 129.9 | 0.319 | [-24.74, 18.98] | ns |
| U-wire-UploadResultDataMessage-upload-as-wt5 | corpus | encode-transport | retain | 139.4 | 137.8 | -1.589 | [-8.403, 11.25] | ns |

