# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E3 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 64 | 64 | 87 [82-110] | 139 [132-145] | 136 [133-151] | 41.4 | E0 |
| ascii | 1024 | 1024 | 141 [140-153] | 197 [189-211] | 190 [186-235] | 46.3 | E0 |
| ascii | 16384 | 16384 | 1127 [1111-1214] | 1187 [1170-1204] | 1152 [1140-1169] | 40.7 | E0 |
| latin1 | 64 | 128 | 126 [125-129] | 175 [174-176] | 169 [168-188] | 40.2 | E0 |
| latin1 | 1024 | 2048 | 908 [883-919] | 501 [486-527] | 490 [480-529] | 41.3 | E3 |
| latin1 | 16384 | 32768 | 13334 [13140-13398] | 5603 [5525-5770] | 5554 [5502-5849] | 39.9 | E3 |
| wide | 64 | 192 | 179 [177-183] | 196 [187-199] | 186 [180-222] | 40.1 | E0 |
| wide | 1024 | 3072 | 1566 [1535-1603] | 632 [628-641] | 633 [618-673] | 41.0 | E1 |
| wide | 16384 | 49152 | 23976 [23753-24936] | 7713 [7646-8583] | 7702 [7586-7817] | 45.0 | E3 |
