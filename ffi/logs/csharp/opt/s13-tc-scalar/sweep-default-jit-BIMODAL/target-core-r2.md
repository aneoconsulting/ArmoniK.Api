# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: Vector512.IsHardwareAccelerated=False. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1R | pin | fastest |
|---|---:|---:|---:|---:|---:|---|
| ascii | 16 | 16 | 75 [73-92] | 705 [695-727] | 42.0 | E0 |
| ascii | 32 | 32 | 75 [74-108] | 700 [697-734] | 75.6 | E0 |
| ascii | 40 | 40 | 75 [74-96] | 729 [704-807] | 39.8 | E0 |
| ascii | 48 | 48 | 74 [74-75] | 702 [698-709] | 40.2 | E0 |
| ascii | 64 | 64 | 74 [74-75] | 700 [697-709] | 39.9 | E0 |
| ascii | 96 | 96 | 76 [76-88] | 700 [697-703] | 40.0 | E0 |
| ascii | 128 | 128 | 80 [79-83] | 718 [711-729] | 40.0 | E0 |
| ascii | 192 | 192 | 83 [82-86] | 731 [716-745] | 40.9 | E0 |
| ascii | 256 | 256 | 86 [85-87] | 721 [717-727] | 39.9 | E0 |
| ascii | 1024 | 1024 | 131 [129-133] | 782 [762-803] | 40.5 | E0 |
| ascii | 16384 | 16384 | 1138 [1132-1150] | 1731 [1706-1746] | 41.7 | E0 |
| latin1 | 16 | 32 | 81 [80-82] | 719 [712-721] | 38.4 | E0 |
| latin1 | 32 | 64 | 92 [91-92] | 719 [716-723] | 40.1 | E0 |
| latin1 | 40 | 80 | 99 [99-99] | 731 [729-735] | 39.9 | E0 |
| latin1 | 48 | 96 | 106 [105-107] | 727 [723-749] | 39.9 | E0 |
| latin1 | 64 | 128 | 120 [119-121] | 743 [737-773] | 40.6 | E0 |
| latin1 | 96 | 192 | 146 [146-149] | 747 [744-762] | 39.8 | E0 |
| latin1 | 128 | 256 | 172 [169-172] | 764 [760-767] | 40.6 | E0 |
| latin1 | 192 | 384 | 227 [218-231] | 814 [781-834] | 42.4 | E0 |
| latin1 | 256 | 512 | 270 [268-272] | 803 [795-810] | 39.5 | E0 |
| latin1 | 1024 | 2048 | 871 [867-873] | 1043 [1039-1048] | 40.5 | E0 |
| latin1 | 16384 | 32768 | 13245 [13168-13282] | 6076 [6070-6094] | 38.5 | E1R |
| wide | 16 | 48 | 91 [90-91] | 739 [738-742] | 40.0 | E0 |
| wide | 32 | 96 | 114 [114-115] | 743 [741-758] | 39.7 | E0 |
| wide | 40 | 120 | 125 [125-126] | 760 [758-763] | 39.8 | E0 |
| wide | 48 | 144 | 144 [142-147] | 777 [760-842] | 38.5 | E0 |
| wide | 64 | 192 | 169 [165-170] | 798 [776-819] | 45.4 | E0 |
| wide | 96 | 288 | 208 [206-211] | 784 [774-784] | 41.3 | E0 |
| wide | 128 | 384 | 250 [249-250] | 791 [789-794] | 39.5 | E0 |
| wide | 192 | 576 | 341 [340-341] | 819 [817-821] | 39.7 | E0 |
| wide | 256 | 768 | 434 [429-444] | 863 [858-872] | 39.8 | E0 |
| wide | 1024 | 3072 | 1537 [1514-1602] | 1233 [1196-1329] | 43.3 | E1R |
| wide | 16384 | 49152 | 24210 [23820-25856] | 8638 [8346-9172] | 39.5 | E1R |
| astral | 16 | 32 | 94 [93-95] | 742 [738-762] | 40.9 | E0 |
| astral | 32 | 64 | 117 [116-120] | 784 [773-795] | 43.2 | E0 |
| astral | 40 | 80 | 129 [129-131] | 792 [790-794] | 48.5 | E0 |
| astral | 48 | 96 | 142 [141-143] | 804 [798-810] | 40.8 | E0 |
| astral | 64 | 128 | 171 [167-173] | 835 [828-866] | 40.7 | E0 |
| astral | 96 | 192 | 228 [225-235] | 895 [884-907] | 41.5 | E0 |
| astral | 128 | 256 | 278 [272-296] | 934 [930-946] | 41.5 | E0 |
| astral | 192 | 384 | 369 [369-371] | 1042 [1033-1043] | 40.5 | E0 |
| astral | 256 | 512 | 470 [467-475] | 1149 [1138-1159] | 38.8 | E0 |
| astral | 1024 | 2048 | 1628 [1624-1635] | 2348 [2337-2360] | 41.0 | E0 |
| astral | 16384 | 32768 | 23934 [23661-25115] | 26703 [26429-28213] | 38.5 | E0 |
