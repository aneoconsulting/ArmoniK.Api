# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1R | pin | fastest |
|---|---:|---:|---:|---:|---:|---|
| ascii | 16 | 16 | 84 [81-110] | 135 [133-145] | 39.9 | E0 |
| ascii | 32 | 32 | 90 [85-120] | 140 [137-143] | 42.2 | E0 |
| ascii | 40 | 40 | 85 [82-87] | 139 [135-159] | 37.9 | E0 |
| ascii | 48 | 48 | 82 [81-84] | 136 [135-141] | 40.5 | E0 |
| ascii | 64 | 64 | 83 [82-83] | 137 [136-138] | 38.6 | E0 |
| ascii | 96 | 96 | 84 [84-87] | 137 [137-155] | 36.9 | E0 |
| ascii | 128 | 128 | 89 [89-92] | 144 [144-152] | 38.7 | E0 |
| ascii | 192 | 192 | 91 [91-92] | 147 [146-152] | 39.2 | E0 |
| ascii | 256 | 256 | 93 [93-94] | 150 [149-169] | 38.4 | E0 |
| ascii | 1024 | 1024 | 133 [133-137] | 190 [190-196] | 36.4 | E0 |
| ascii | 16384 | 16384 | 1113 [1112-1114] | 1124 [1122-1166] | 38.8 | E0 |
| latin1 | 16 | 32 | 90 [89-90] | 150 [149-151] | 38.7 | E0 |
| latin1 | 32 | 64 | 103 [103-103] | 158 [158-158] | 38.4 | E0 |
| latin1 | 40 | 80 | 112 [110-112] | 168 [168-169] | 38.8 | E0 |
| latin1 | 48 | 96 | 117 [117-118] | 159 [159-160] | 38.4 | E0 |
| latin1 | 64 | 128 | 132 [130-143] | 172 [170-191] | 38.5 | E0 |
| latin1 | 96 | 192 | 159 [159-162] | 184 [180-190] | 38.6 | E0 |
| latin1 | 128 | 256 | 183 [182-188] | 191 [189-193] | 38.3 | E0 |
| latin1 | 192 | 384 | 235 [233-238] | 215 [210-236] | 38.3 | E1R |
| latin1 | 256 | 512 | 283 [282-296] | 233 [232-235] | 38.3 | E1R |
| latin1 | 1024 | 2048 | 883 [881-884] | 479 [478-480] | 39.1 | E1R |
| latin1 | 16384 | 32768 | 13340 [12905-13483] | 5619 [5478-5705] | 36.8 | E1R |
| wide | 16 | 48 | 108 [107-131] | 161 [160-163] | 36.5 | E0 |
| wide | 32 | 96 | 132 [130-146] | 171 [170-175] | 39.7 | E0 |
| wide | 40 | 120 | 141 [140-142] | 185 [181-192] | 38.5 | E0 |
| wide | 48 | 144 | 155 [154-155] | 180 [179-181] | 38.6 | E0 |
| wide | 64 | 192 | 176 [175-176] | 186 [185-190] | 38.3 | E0 |
| wide | 96 | 288 | 222 [222-222] | 200 [199-200] | 38.6 | E1R |
| wide | 128 | 384 | 262 [262-263] | 218 [218-219] | 38.4 | E1R |
| wide | 192 | 576 | 348 [348-349] | 244 [244-245] | 38.8 | E1R |
| wide | 256 | 768 | 440 [434-450] | 275 [271-282] | 38.7 | E1R |
| wide | 1024 | 3072 | 1510 [1501-1513] | 627 [626-628] | 38.3 | E1R |
| wide | 16384 | 49152 | 23072 [22825-23135] | 7790 [7731-7846] | 38.0 | E1R |
| astral | 16 | 32 | 98 [96-98] | 155 [154-156] | 38.7 | E0 |
| astral | 32 | 64 | 120 [117-121] | 185 [182-187] | 38.4 | E0 |
| astral | 40 | 80 | 131 [130-134] | 196 [192-197] | 38.9 | E0 |
| astral | 48 | 96 | 141 [139-143] | 205 [203-206] | 36.7 | E0 |
| astral | 64 | 128 | 168 [165-173] | 237 [235-239] | 38.4 | E0 |
| astral | 96 | 192 | 225 [224-226] | 285 [283-293] | 39.0 | E0 |
| astral | 128 | 256 | 256 [254-259] | 336 [333-340] | 39.2 | E0 |
| astral | 192 | 384 | 344 [342-351] | 452 [443-462] | 38.8 | E0 |
| astral | 256 | 512 | 440 [435-458] | 557 [542-570] | 38.6 | E0 |
| astral | 1024 | 2048 | 1457 [1449-1458] | 1730 [1728-1739] | 37.4 | E0 |
| astral | 16384 | 32768 | 24483 [24380-25104] | 25624 [25481-25785] | 37.7 | E0 |
