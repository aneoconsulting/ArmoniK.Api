# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1R | pin | fastest |
|---|---:|---:|---:|---:|---:|---|
| ascii | 16 | 16 | 87 [86-132] | 138 [137-145] | 38.8 | E0 |
| ascii | 32 | 32 | 93 [92-133] | 146 [139-151] | 41.7 | E0 |
| ascii | 40 | 40 | 95 [93-95] | 144 [142-162] | 38.5 | E0 |
| ascii | 48 | 48 | 95 [94-97] | 141 [141-143] | 40.5 | E0 |
| ascii | 64 | 64 | 96 [94-97] | 147 [141-148] | 36.6 | E0 |
| ascii | 96 | 96 | 93 [93-93] | 142 [141-142] | 38.5 | E0 |
| ascii | 128 | 128 | 94 [94-95] | 149 [149-150] | 38.7 | E0 |
| ascii | 192 | 192 | 96 [95-97] | 154 [152-158] | 36.6 | E0 |
| ascii | 256 | 256 | 109 [107-115] | 156 [153-157] | 38.5 | E0 |
| ascii | 1024 | 1024 | 148 [146-150] | 202 [197-206] | 38.6 | E0 |
| ascii | 16384 | 16384 | 1229 [1215-1273] | 1151 [1135-1181] | 39.7 | E1R |
| latin1 | 16 | 32 | 96 [95-112] | 158 [155-178] | 38.4 | E0 |
| latin1 | 32 | 64 | 109 [107-111] | 165 [163-169] | 36.5 | E0 |
| latin1 | 40 | 80 | 113 [112-113] | 172 [170-173] | 38.4 | E0 |
| latin1 | 48 | 96 | 119 [119-119] | 164 [164-166] | 38.5 | E0 |
| latin1 | 64 | 128 | 133 [133-133] | 178 [178-182] | 38.6 | E0 |
| latin1 | 96 | 192 | 161 [160-162] | 190 [189-192] | 36.6 | E0 |
| latin1 | 128 | 256 | 185 [184-188] | 200 [199-204] | 39.0 | E0 |
| latin1 | 192 | 384 | 235 [233-237] | 219 [218-222] | 38.6 | E1R |
| latin1 | 256 | 512 | 283 [282-284] | 238 [238-239] | 38.3 | E1R |
| latin1 | 1024 | 2048 | 909 [904-910] | 488 [485-490] | 38.7 | E1R |
| latin1 | 16384 | 32768 | 13461 [13296-13580] | 5537 [5496-5552] | 37.6 | E1R |
| wide | 16 | 48 | 107 [106-108] | 171 [164-178] | 38.5 | E0 |
| wide | 32 | 96 | 126 [126-127] | 172 [171-175] | 38.2 | E0 |
| wide | 40 | 120 | 141 [137-147] | 191 [187-195] | 37.1 | E0 |
| wide | 48 | 144 | 155 [154-157] | 185 [184-187] | 38.7 | E0 |
| wide | 64 | 192 | 174 [174-175] | 192 [192-192] | 36.9 | E0 |
| wide | 96 | 288 | 220 [214-224] | 211 [205-214] | 38.8 | E1R |
| wide | 128 | 384 | 259 [253-266] | 221 [219-221] | 38.5 | E1R |
| wide | 192 | 576 | 343 [341-353] | 249 [249-251] | 38.8 | E1R |
| wide | 256 | 768 | 438 [436-440] | 279 [278-280] | 38.6 | E1R |
| wide | 1024 | 3072 | 1444 [1427-1450] | 641 [634-682] | 38.3 | E1R |
| wide | 16384 | 49152 | 22885 [22774-23188] | 7820 [7781-8010] | 59.7 | E1R |
| astral | 16 | 32 | 108 [107-108] | 161 [159-174] | 39.3 | E0 |
| astral | 32 | 64 | 128 [128-131] | 188 [187-190] | 38.6 | E0 |
| astral | 40 | 80 | 140 [139-141] | 198 [196-199] | 36.7 | E0 |
| astral | 48 | 96 | 150 [150-151] | 209 [208-209] | 38.5 | E0 |
| astral | 64 | 128 | 173 [172-175] | 243 [242-245] | 39.2 | E0 |
| astral | 96 | 192 | 226 [225-226] | 289 [289-289] | 38.3 | E0 |
| astral | 128 | 256 | 268 [267-269] | 338 [337-339] | 38.4 | E0 |
| astral | 192 | 384 | 360 [355-365] | 467 [451-470] | 38.6 | E0 |
| astral | 256 | 512 | 440 [437-471] | 574 [551-654] | 38.2 | E0 |
| astral | 1024 | 2048 | 1513 [1489-1559] | 1756 [1740-1760] | 38.1 | E0 |
| astral | 16384 | 32768 | 23946 [23688-24046] | 25817 [25557-26107] | 35.9 | E0 |
