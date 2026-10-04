# D21 string-length sweep: process CPU per encode (ns), median [min-max] over 6 rounds, one process, paths interleaved and rotated per round

CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer. `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: all identical.

| content | chars | UTF-8 bytes | E0 | E1 | E2 | pin | fastest |
|---|---:|---:|---:|---:|---:|---:|---|
| ascii | 4 | 4 | 76 [70-106] | 125 [122-133] | 101 [98-110] | 43.0 | E0 |
| ascii | 64 | 64 | 76 [75-86] | 135 [132-151] | 108 [105-136] | 48.8 | E0 |
| wide | 4 | 12 | 80 [77-82] | 132 [128-139] | 111 [106-112] | 45.9 | E0 |
| wide | 64 | 192 | 169 [164-179] | 191 [182-195] | 200 [194-233] | 43.2 | E0 |
