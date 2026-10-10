# rows by direction, mode and change: fwd delta (= -(ak_enc_reset removed + ak_dec_reset_* removed)); rev and grow unchanged on every row
| dir | mode | fwd change | ak_enc_reset removed | ak_dec_reset removed | rows |
|---|---|---:|---:|---:|---:|
| decode | drop | -1 | 0 | 1 | 158 |
| decode | retain | -1 | 0 | 1 | 136 |
| decode-read | drop | -1 | 0 | 1 | 158 |
| decode-read | retain | -1 | 0 | 1 | 136 |
| decode-reencode | drop | -2 | 1 | 1 | 92 |
| decode-reencode | retain | -2 | 1 | 1 | 92 |
| encode | drop | -1 | 1 | 0 | 22 |
| encode | retain | -1 | 1 | 0 | 22 |
| encode-core | drop | -1 | 1 | 0 | 22 |
| encode-core | retain | -1 | 1 | 0 | 22 |
| encode-core-hot | drop | -1 | 1 | 0 | 29 |
| encode-core-hot | retain | -1 | 1 | 0 | 29 |
| encode-hot | drop | -1 | 1 | 0 | 114 |
| encode-hot | retain | -1 | 1 | 0 | 114 |
| encode-transport | drop | -1 | 1 | 0 | 22 |
| encode-transport | retain | -1 | 1 | 0 | 22 |
| encode-transport-hot | drop | -1 | 1 | 0 | 22 |
| encode-transport-hot | retain | -1 | 1 | 0 | 29 |
1241 rows compared, 0 break the rule
