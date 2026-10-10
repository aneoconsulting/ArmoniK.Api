# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=True. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity NOT checked: the loaded core is the s14 measurement stub (wrong UTF-16 output by design).

| content | chars | UTF-8 bytes | E0 | E1R | pin | fastest |
|---|---:|---:|---:|---:|---:|---|
| ascii | 40 | 40 | 80 [78-102] | 102 [100-110] | 39.3 | E0 |
| ascii | 48 | 48 | 84 [77-116] | 103 [101-114] | 42.7 | E0 |
| latin1 | 40 | 80 | 94 [94-99] | 103 [100-112] | 39.5 | E0 |
| latin1 | 48 | 96 | 100 [97-101] | 103 [101-115] | 39.5 | E0 |
