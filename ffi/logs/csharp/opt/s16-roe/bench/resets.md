# s16: one reset call alone from C# (the default P/Invoke binding), back to back, ns per call: the median over processes of each process's median over its rounds [min-max of the process medians]. CONTAINER INSTRUMENTATION.

| call | options | processes | ns per call |
|---|---|---:|---|
| ak_dec_reset_DualResponse | NULL | 15 | 3.6 [3.2, 3.9] |
| ak_dec_reset_DualResponse | retain | 15 | 6.9 [6.4, 7.5] |
| ak_dec_reset_ListMetricsResponse | NULL | 15 | 3.4 [3.2, 3.6] |
| ak_dec_reset_ListMetricsResponse | retain | 15 | 5.6 [5.2, 6.0] |
| ak_dec_reset_ListProbeResponse | NULL | 15 | 3.2 [3.1, 3.7] |
| ak_dec_reset_ListProbeResponse | retain | 15 | 6.5 [6.2, 7.2] |
| ak_dec_reset_ListResultsResponse | NULL | 15 | 3.2 [3.0, 3.6] |
| ak_dec_reset_ListResultsResponse | retain | 15 | 7.5 [7.4, 8.0] |
| ak_dec_reset_ListTaskSummaryResponse | NULL | 15 | 3.8 [3.2, 4.4] |
| ak_dec_reset_ListTaskSummaryResponse | retain | 15 | 10.8 [10.4, 11.4] |
| ak_dec_reset_ListTasksDetailedResponse | NULL | 15 | 4.1 [3.4, 4.6] |
| ak_dec_reset_ListTasksDetailedResponse | retain | 15 | 34.8 [31.7, 38.4] |
| ak_dec_reset_UploadResultDataMessage | NULL | 15 | 3.2 [3.1, 3.7] |
| ak_dec_reset_UploadResultDataMessage | retain | 15 | 5.2 [5.0, 5.7] |
| ak_enc_reset | - | 15 | 5.3 [4.9, 5.8] |
