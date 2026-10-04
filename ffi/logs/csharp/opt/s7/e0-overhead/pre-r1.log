# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 73 [69-89] | 136 [133-137] | 102 [100-105] | 41.3 | E0 |
| ascii | 64 | 64 | 79 [74-98] | 145 [142-149] | 113 [110-145] | 48.2 | E0 |
| wide | 4 | 12 | 74 [73-75] | 141 [139-146] | 109 [107-110] | 44.4 | E0 |
| wide | 64 | 192 | 162 [160-163] | 202 [194-216] | 196 [195-202] | 43.1 | E0 |
