# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 91 [85-108] | 133 [129-143] | 113 [109-117] | 40.3 | E0 |
| ascii | 64 | 64 | 90 [89-100] | 140 [139-147] | 120 [116-150] | 46.7 | E0 |
| wide | 4 | 12 | 91 [90-91] | 137 [136-139] | 118 [117-118] | 43.6 | E0 |
| wide | 64 | 192 | 179 [179-182] | 194 [192-235] | 205 [203-233] | 46.2 | E0 |
