# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 105 [102-124] | 152 [146-156] | 126 [122-151] | 72.8 | E0 |
| ascii | 64 | 64 | 102 [102-107] | 155 [155-175] | 130 [128-159] | 40.1 | E0 |
| wide | 4 | 12 | 107 [105-112] | 158 [153-167] | 132 [128-135] | 41.7 | E0 |
| wide | 64 | 192 | 187 [186-206] | 208 [207-209] | 212 [211-212] | 39.8 | E0 |
