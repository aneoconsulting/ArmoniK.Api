# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: SIMDUTF_FORCE_IMPLEMENTATION=westmere, Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E3 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 64 | 64 | 86 [82-103] | 132 [131-134] | 134 [130-137] | 44.4 | E0 |
| ascii | 1024 | 1024 | 134 [132-147] | 215 [213-223] | 210 [204-259] | 47.5 | E0 |
| ascii | 16384 | 16384 | 1113 [1096-1144] | 1475 [1451-1483] | 1458 [1446-1490] | 41.7 | E0 |
| latin1 | 64 | 128 | 125 [123-128] | 184 [180-202] | 172 [170-175] | 38.6 | E0 |
| latin1 | 1024 | 2048 | 814 [789-846] | 567 [545-650] | 566 [546-579] | 39.4 | E3 |
| latin1 | 16384 | 32768 | 12087 [11777-12704] | 6373 [6250-6971] | 6435 [6398-6768] | 38.9 | E1 |
| wide | 64 | 192 | 173 [172-181] | 206 [202-208] | 185 [182-191] | 39.2 | E0 |
| wide | 1024 | 3072 | 1538 [1513-1553] | 606 [596-622] | 591 [588-599] | 40.1 | E3 |
| wide | 16384 | 49152 | 23971 [23594-24460] | 7153 [7008-7291] | 7177 [7069-7227] | 38.2 | E1 |
