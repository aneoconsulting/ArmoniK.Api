# armonik-transport-ffi

A C ABI over [`armonik-transport`](../armonik-transport), for host applications that cannot link
Rust directly.

Configuration, TLS, mTLS and proxying all come from `armonik-transport`. This crate adds the
boundary: result codes, owned and borrowed buffers, reference-counted handles, panic guards, and the
one key/value encoding every list of pairs travels in.

## Features

`error-locations` keeps the locations that errors carry in the message that crosses the ABI: the
` [file.rs:12:34]` of a Rust source, and the ` at line 1 column 56` position `serde_json` names in
the configuration document.

Off by default, and that is a disclosure rule rather than a matter of taste: a released build says
what went wrong, never whereabouts in the source it happened. Being open source changes nothing -
what someone can find by reading a repository is not what a library volunteers in a log. What the
messages say is the same either way; only the coordinates go.

It is a build-time choice rather than an option, so a released library carries neither the branch nor
the decision. The ABI is identical either way, which is what lets the two builds be interchangeable:
a consumer picks one by which library it ships, not by how it calls.
