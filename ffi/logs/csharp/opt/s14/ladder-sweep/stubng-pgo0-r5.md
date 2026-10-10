# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=True. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity NOT checked: the loaded core is the s14 measurement stub (wrong UTF-16 output by design).

| content | chars | UTF-8 bytes | E0 | E1R | pin | fastest |
|---|---:|---:|---:|---:|---:|---|
| ascii | 40 | 40 | 80 [78-102] | 106 [101-115] | 41.6 | E0 |
| ascii | 48 | 48 | 82 [77-115] | 100 [100-113] | 39.9 | E0 |
| latin1 | 40 | 80 | 112 [111-113] | 100 [99-110] | 38.9 | E1R |
| latin1 | 48 | 96 | 120 [115-125] | 99 [98-106] | 40.5 | E1R |
