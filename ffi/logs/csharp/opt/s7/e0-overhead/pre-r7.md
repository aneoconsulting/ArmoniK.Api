# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 83 [79-102] | 130 [128-134] | 108 [105-109] | 40.2 | E0 |
| ascii | 64 | 64 | 84 [81-92] | 140 [138-152] | 114 [111-188] | 45.5 | E0 |
| wide | 4 | 12 | 86 [86-87] | 138 [137-139] | 113 [112-114] | 39.9 | E0 |
| wide | 64 | 192 | 182 [179-186] | 192 [190-208] | 199 [197-203] | 41.0 | E0 |
