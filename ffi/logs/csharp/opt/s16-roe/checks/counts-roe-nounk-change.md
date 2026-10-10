# rows by direction, mode and change: fwd delta (= -(ak_enc_reset removed + ak_dec_reset_* removed)); rev and grow unchanged on every row
| dir | mode | fwd change | ak_enc_reset removed | ak_dec_reset removed | rows |
|---|---|---:|---:|---:|---:|
| decode | no-unknown | 0 | 0 | 0 | 158 |
| decode-read | no-unknown | 0 | 0 | 0 | 158 |
| decode-reencode | no-unknown | -1 | 1 | 0 | 92 |
| encode | no-unknown | -1 | 1 | 0 | 22 |
| encode-core | no-unknown | -1 | 1 | 0 | 22 |
| encode-core-hot | no-unknown | -1 | 1 | 0 | 29 |
| encode-hot | no-unknown | -1 | 1 | 0 | 114 |
| encode-transport | no-unknown | -1 | 1 | 0 | 22 |
| encode-transport-hot | no-unknown | -1 | 1 | 0 | 29 |
646 rows compared, 0 break the rule
