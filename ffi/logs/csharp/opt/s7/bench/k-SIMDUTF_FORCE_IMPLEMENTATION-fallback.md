# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: SIMDUTF_FORCE_IMPLEMENTATION=fallback, Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E3 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 64 | 64 | 92 [83-107] | 150 [145-158] | 147 [143-158] | 43.8 | E0 |
| ascii | 1024 | 1024 | 135 [132-147] | 458 [456-473] | 463 [449-566] | 54.8 | E0 |
| ascii | 16384 | 16384 | 1093 [1075-1097] | 5227 [5197-5266] | 5256 [5166-5362] | 45.5 | E0 |
| latin1 | 64 | 128 | 129 [129-133] | 223 [220-244] | 211 [209-212] | 40.0 | E0 |
| latin1 | 1024 | 2048 | 798 [793-835] | 1466 [1451-1470] | 1452 [1438-1476] | 39.9 | E0 |
| latin1 | 16384 | 32768 | 11923 [11701-13754] | 21119 [20844-21711] | 21108 [20918-27744] | 40.9 | E0 |
| wide | 64 | 192 | 178 [177-179] | 268 [265-275] | 257 [252-274] | 40.0 | E0 |
| wide | 1024 | 3072 | 1544 [1517-1561] | 2141 [2106-2287] | 2112 [2103-2126] | 39.0 | E0 |
| wide | 16384 | 49152 | 23267 [23051-23932] | 31623 [31042-31845] | 31756 [31071-31999] | 42.3 | E0 |
