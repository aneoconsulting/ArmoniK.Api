# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 78 [74-94] | 125 [123-128] | 106 [103-110] | 38.9 | E0 |
| ascii | 64 | 64 | 77 [75-83] | 132 [131-138] | 116 [110-145] | 45.3 | E0 |
| wide | 4 | 12 | 77 [76-78] | 132 [130-134] | 111 [109-112] | 42.1 | E0 |
| wide | 64 | 192 | 160 [159-173] | 191 [186-204] | 195 [194-203] | 40.8 | E0 |
