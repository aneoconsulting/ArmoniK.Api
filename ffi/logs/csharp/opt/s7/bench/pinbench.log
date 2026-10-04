# D21 step 7: pinning N strings apart from the codec, process CPU ns per string, median [min-max] over 6 rounds; minflt and GC collections per op (N strings)

CONTAINER INSTRUMENTATION. `live` = N GCHandle.Alloc(Pinned) then N Free (E1); `chunk64` = Alloc/Free 64 at a time (E1C); `fixed64` = one recursive frame per string with `fixed`, 64 deep (E1R's frames alone).

| N | live | chunk64 | fixed64 | live minflt/op | live gen0/op |
|---:|---:|---:|---:|---:|---:|
| 1 | 45.2 [41.1-64.1] | 40.6 [40.3-51.6] | 4.1 [4.0-4.4] | 4.77E-06 | 0 |
| 16 | 44.1 [43.5-52.7] | 44.5 [43.6-52.5] | 2.0 [2.0-2.2] | 0 | 0 |
| 64 | 43.8 [43.5-44.0] | 44.7 [44.3-51.4] | 7.7 [7.6-7.7] | 0 | 0 |
| 256 | 75.8 [75.1-78.6] | 44.4 [44.0-48.8] | 7.9 [7.7-7.9] | 0 | 0 |
| 1024 | 85.3 [84.5-85.8] | 44.9 [44.6-45.4] | 7.8 [7.8-7.9] | 0 | 0 |
| 5000 | 89.3 [89.2-89.9] | 45.6 [44.9-45.6] | 7.8 [7.7-8.0] | 0 | 0 |
| 17167 | 92.8 [91.7-93.1] | 46.6 [46.2-46.9] | 7.8 [7.6-8.0] | 0 | 0 |
| 26267 | 94.7 [93.3-97.7] | 46.1 [45.7-46.8] | 7.8 [7.8-7.8] | 0 | 0 |
