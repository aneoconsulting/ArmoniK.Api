# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=True. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity NOT checked: the loaded core is the s14 measurement stub (wrong UTF-16 output by design).

| content | chars | UTF-8 bytes | E0 | E1R | pin | fastest |
|---|---:|---:|---:|---:|---:|---|
| ascii | 40 | 40 | 80 [78-104] | 102 [101-114] | 40.8 | E0 |
| ascii | 48 | 48 | 78 [77-111] | 102 [100-109] | 39.9 | E0 |
| latin1 | 40 | 80 | 100 [96-120] | 105 [102-114] | 40.4 | E0 |
| latin1 | 48 | 96 | 100 [99-101] | 101 [100-102] | 40.2 | E0 |
