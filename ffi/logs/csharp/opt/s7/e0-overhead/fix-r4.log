# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 85 [81-101] | 129 [126-134] | 105 [103-105] | 42.4 | E0 |
| ascii | 64 | 64 | 84 [82-92] | 137 [135-141] | 113 [110-140] | 43.9 | E0 |
| wide | 4 | 12 | 86 [85-89] | 136 [133-142] | 111 [110-118] | 45.1 | E0 |
| wide | 64 | 192 | 179 [173-182] | 196 [190-217] | 198 [194-205] | 40.2 | E0 |
