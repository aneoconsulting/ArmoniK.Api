# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 92 [82-105] | 135 [133-138] | 116 [109-127] | 43.1 | E0 |
| ascii | 64 | 64 | 92 [88-99] | 144 [141-147] | 123 [116-165] | 46.9 | E0 |
| wide | 4 | 12 | 82 [82-83] | 139 [136-140] | 117 [116-117] | 43.4 | E0 |
| wide | 64 | 192 | 168 [166-172] | 196 [194-216] | 206 [205-207] | 42.8 | E0 |
