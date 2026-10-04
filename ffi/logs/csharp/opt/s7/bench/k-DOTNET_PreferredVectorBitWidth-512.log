# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: DOTNET_PreferredVectorBitWidth=512, Vector512.IsHardwareAccelerated=True. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E3 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 64 | 64 | 92 [90-110] | 150 [147-155] | 143 [139-156] | 44.9 | E0 |
| ascii | 1024 | 1024 | 147 [137-150] | 216 [213-217] | 212 [197-263] | 46.8 | E0 |
| ascii | 16384 | 16384 | 1302 [1260-1344] | 1377 [1237-1535] | 1247 [1207-1389] | 71.2 | E3 |
| latin1 | 64 | 128 | 134 [130-177] | 191 [188-233] | 188 [181-192] | 40.5 | E0 |
| latin1 | 1024 | 2048 | 852 [845-855] | 520 [519-522] | 512 [509-516] | 40.8 | E3 |
| latin1 | 16384 | 32768 | 12796 [12658-13405] | 6054 [5857-6140] | 5924 [5874-6240] | 41.9 | E3 |
| wide | 64 | 192 | 182 [181-185] | 204 [203-207] | 193 [190-214] | 45.9 | E0 |
| wide | 1024 | 3072 | 1624 [1621-1664] | 670 [665-683] | 657 [655-672] | 42.7 | E3 |
| wide | 16384 | 49152 | 24335 [24095-24488] | 8185 [8157-8201] | 8160 [8127-8205] | 39.6 | E3 |
