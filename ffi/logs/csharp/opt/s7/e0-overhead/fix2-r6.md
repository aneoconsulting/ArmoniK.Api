# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 87 [84-108] | 133 [130-137] | 112 [110-122] | 40.1 | E0 |
| ascii | 64 | 64 | 92 [89-99] | 147 [141-155] | 123 [119-147] | 44.6 | E0 |
| wide | 4 | 12 | 93 [92-108] | 140 [138-153] | 120 [118-130] | 40.6 | E0 |
| wide | 64 | 192 | 180 [175-183] | 199 [196-211] | 214 [202-232] | 40.6 | E0 |
