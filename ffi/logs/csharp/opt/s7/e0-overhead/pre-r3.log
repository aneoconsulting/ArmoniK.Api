# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 74 [72-89] | 123 [122-125] | 97 [94-97] | 40.3 | E0 |
| ascii | 64 | 64 | 76 [74-88] | 127 [126-136] | 101 [100-182] | 40.0 | E0 |
| wide | 4 | 12 | 75 [75-76] | 126 [125-127] | 101 [101-104] | 39.5 | E0 |
| wide | 64 | 192 | 164 [162-166] | 183 [180-201] | 188 [186-193] | 40.5 | E0 |
