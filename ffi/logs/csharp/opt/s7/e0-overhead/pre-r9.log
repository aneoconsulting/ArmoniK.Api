# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 92 [91-109] | 144 [141-146] | 116 [114-121] | 49.7 | E0 |
| ascii | 64 | 64 | 93 [92-95] | 151 [150-156] | 120 [118-155] | 41.1 | E0 |
| wide | 4 | 12 | 98 [95-103] | 150 [148-150] | 123 [120-132] | 40.8 | E0 |
| wide | 64 | 192 | 174 [174-196] | 200 [199-201] | 204 [203-207] | 40.0 | E0 |
