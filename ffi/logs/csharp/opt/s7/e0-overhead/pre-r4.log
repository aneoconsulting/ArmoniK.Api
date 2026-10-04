# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 76 [73-94] | 129 [125-135] | 102 [100-111] | 41.5 | E0 |
| ascii | 64 | 64 | 76 [75-77] | 137 [132-165] | 111 [108-141] | 49.6 | E0 |
| wide | 4 | 12 | 78 [75-88] | 136 [133-140] | 114 [110-119] | 42.5 | E0 |
| wide | 64 | 192 | 164 [162-171] | 189 [185-199] | 202 [196-217] | 41.3 | E0 |
