# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: DOTNET_EnableAVX512F=0, Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E3 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 64 | 64 | 91 [86-124] | 138 [137-141] | 130 [128-132] | 40.6 | E0 |
| ascii | 1024 | 1024 | 144 [141-158] | 201 [195-217] | 193 [182-232] | 45.9 | E0 |
| ascii | 16384 | 16384 | 1056 [1028-1100] | 1176 [1154-1187] | 1141 [1128-1162] | 65.8 | E0 |
| latin1 | 64 | 128 | 129 [126-246] | 179 [174-206] | 166 [164-172] | 40.2 | E0 |
| latin1 | 1024 | 2048 | 812 [793-843] | 489 [484-494] | 482 [475-483] | 40.5 | E3 |
| latin1 | 16384 | 32768 | 11754 [11676-11940] | 5551 [5488-5984] | 5523 [5487-5592] | 38.8 | E3 |
| wide | 64 | 192 | 176 [175-181] | 191 [189-203] | 176 [174-179] | 40.3 | E0 |
| wide | 1024 | 3072 | 1561 [1536-1569] | 636 [624-641] | 624 [609-772] | 41.6 | E3 |
| wide | 16384 | 49152 | 23793 [23651-23929] | 7609 [7589-8040] | 7650 [7586-7730] | 38.8 | E1 |
