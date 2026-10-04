# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 75 [70-92] | 121 [120-128] | 101 [98-104] | 40.0 | E0 |
| ascii | 64 | 64 | 77 [73-82] | 134 [129-134] | 128 [122-156] | 42.6 | E0 |
| wide | 4 | 12 | 74 [73-96] | 127 [126-150] | 108 [106-121] | 40.2 | E0 |
| wide | 64 | 192 | 163 [160-168] | 185 [181-202] | 193 [191-197] | 40.4 | E0 |
