# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=True. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity NOT checked: the loaded core is the s14 measurement stub (wrong UTF-16 output by design).

| content | chars | UTF-8 bytes | E0 | E1R | pin | fastest |
|---|---:|---:|---:|---:|---:|---|
| ascii | 40 | 40 | 79 [76-95] | 95 [91-100] | 45.6 | E0 |
| ascii | 48 | 48 | 77 [74-114] | 91 [89-105] | 60.7 | E0 |
| latin1 | 40 | 80 | 106 [94-147] | 100 [92-118] | 44.6 | E1R |
| latin1 | 48 | 96 | 97 [95-98] | 89 [88-90] | 45.4 | E1R |
