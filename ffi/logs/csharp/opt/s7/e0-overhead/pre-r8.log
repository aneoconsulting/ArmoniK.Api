# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 74 [71-99] | 128 [124-130] | 105 [103-107] | 40.4 | E0 |
| ascii | 64 | 64 | 82 [81-91] | 139 [137-142] | 111 [109-141] | 43.7 | E0 |
| wide | 4 | 12 | 84 [76-88] | 132 [130-141] | 112 [110-127] | 40.9 | E0 |
| wide | 64 | 192 | 161 [158-165] | 186 [183-209] | 202 [198-208] | 39.9 | E0 |
