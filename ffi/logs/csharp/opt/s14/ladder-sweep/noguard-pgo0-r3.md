# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=True. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1R | pin | fastest |
|---|---:|---:|---:|---:|---:|---|
| ascii | 40 | 40 | 86 [80-109] | 116 [115-120] | 42.5 | E0 |
| ascii | 48 | 48 | 82 [78-111] | 114 [113-120] | 42.1 | E0 |
| latin1 | 40 | 80 | 95 [95-101] | 117 [116-135] | 41.9 | E0 |
| latin1 | 48 | 96 | 100 [99-104] | 117 [116-121] | 42.3 | E0 |
