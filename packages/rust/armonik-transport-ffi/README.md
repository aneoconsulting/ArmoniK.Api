# armonik-transport-ffi

A C ABI over [`armonik-transport`](../armonik-transport), for host applications that cannot link
Rust directly.

Configuration, TLS, mTLS and proxying all come from `armonik-transport`. This crate adds the
boundary: result codes, owned and borrowed buffers, reference-counted handles, panic guards, and the
one key/value encoding every list of pairs travels in.
