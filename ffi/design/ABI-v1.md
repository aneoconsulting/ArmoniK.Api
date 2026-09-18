# The C ABI, v1

**Status: draft for agreement (W1).** This is the specification every slice is
built against. Until it is agreed, no slice starts; once it is agreed, a slice
that disagrees with it raises a finding rather than diverging quietly.

**It is called v1 because it is meant to be the only one.** The version is not a
hedge against this document being provisional: it is there so that a future shape
this design cannot absorb has a name (`ABI-v2.md`, in its own document) rather
than arriving as an amendment that silently changes what a shipped binding
expects. Within v1, symbols are unsuffixed and `ak_abi_version()` is what a host
checks at load. If a v2 ever has to coexist with v1 in one process, it takes a
symbol prefix of its own; nothing in v1 reserves one today.

It merges three sources. The [base design](https://claude.ai/code/artifact/d29ed568-05eb-4ded-b22a-b1db669a56fb?sk=k-raOgdhnjAvYmpd0YdSGA)
supplies the layering, the RPC half, lifetime and the streaming contract. The
[C# finding](https://claude.ai/artifact/WYD94FSYuq1Nxjdu6WHbtS?sk=aeQYJo8cccAcZsFgdTXRkQ)
section 4 replaces the codec half, which it measured as a regression in the base
design's form. The [Java finding](https://claude.ai/artifact/YFSNVzYD41C1TsHmANLcBu?sk=MJlBPbq3WV9R4dxdhAv6Og)
section 4 amends that again and, on decode, contradicts it; section 7 below is
where the contradiction is settled rather than averaged.

Names here are illustrative. The generator emits them, and where a name appears
it is to fix a shape, an argument order or an ownership rule.

## 1. The two rules, and how they fit together

**Rule 1, the boundary.** *The host may drive iteration over its own containers.
The host must never need to know the wire format.*

The first half is necessary because only the host knows how its collections are
stored. The second is where the design stops sliding: once a crossing is known to
cost 10 ns one way and 2 ns the other, every measurement argues for moving one
more thing into the cheap direction, and that gradient ends with the host
encoding its own messages, which is the five-implementations problem this exists
to remove. The line is drawn on versioning, not on taste: **a named accessor slot
is safe across a schema change and a positional stream of values is not.** A host
built against an older descriptor has no slot for a new field; a host writing a
positional stream inserts the wrong bytes into the right frame, silently, and
adding a field is the most common thing a protobuf schema does.

**Rule 2, the direction.** *The party that owns the destination drives.* On
encode the destination is the codec's buffer, so the codec drives and the host
supplies pointers. On decode the destination is the host's objects, so the host
drives and the codec supplies an intermediate.

Rule 2 is a mechanism rule and Rule 1 is a boundary rule. They agree everywhere
on encode. On decode they do not, and section 7 settles it with two delivery
families rather than one answer, because the evidence says the right answer
differs by runtime and not by preference.

**Everything else in this document is those rules applied to the descriptor.**
If a case here cannot be derived from the descriptor, it is a defect in the
specification.

## 2. Layers

```
host application
  facade                 hand-written, idiomatic, per language
  binding                GENERATED: field <-> slot mapping, vtables, entry points
  ---------------------- the C ABI, fixed size, does not grow with the schema
  armonik-codec-ffi      generated per-message encoders and decoders
  armonik-ffi            clients, worker, handles
  armonik-transport-ffi  transport, TLS, retry, cancellation
  tonic / hyper / rustls / tokio
```

Two notes that are not obvious from the picture.

**Python has three layers where the others have two.** Its binding is generated
C speaking the CPython API, because the core calls Python primitives rather than
calling back into Python (README 9.1). Everywhere this document says "the host
answers", read "the binding answers": the accessor contract is a contract with a
binding, never with a host language.

**The RPC half does not know the schema.** It moves opaque bytes and dispatches
on a path string, so it serves every RPC unchanged and adding one is a table row.
Only the codec half is generated per message.

## 3. Lifecycle: what a host does, in order

Nothing here is implicit. A host initialises the library, then builds a runtime,
then a context, then a client, and destroys them in the reverse order.

```c
/* ---- once per process, before anything else, codec included ---------------- */
struct ak_init_opts {
  uint32_t abi_version;     /* the version the HOST was generated against */
  uint32_t flags;           /* AK_INIT_OWN_LOGGING, AK_INIT_NO_PANIC_HOOK, ... */
  ak_log_fn log;            /* NULL unless the host takes the process log */
  void     *log_ctx;
};
int32_t     ak_init(const struct ak_init_opts*, ak_err*);
uint32_t    ak_abi_version(void);
const char *ak_build_id(void);

/* ---- the transport stack --------------------------------------------------- */
ak_runtime *ak_runtime_new(const ak_runtime_opts*, ak_err*);
ak_context *ak_context_new(ak_runtime*, ak_bytes_in config_json, ak_err*);
ak_client  *ak_client_new (ak_context*, ak_err*);
void ak_client_destroy(ak_client*);
void ak_context_destroy(ak_context*);
void ak_runtime_destroy(ak_runtime*);

/* ---- the codec, which needs no runtime ------------------------------------- */
ak_enc_ctx *ak_enc_ctx_new(void);   void ak_enc_ctx_free(ak_enc_ctx*);
ak_dec_ctx *ak_dec_ctx_new(void);   void ak_dec_ctx_free(ak_dec_ctx*);
void ak_enc_reset(ak_enc_ctx*);     void ak_dec_reset(ak_dec_ctx*);
```

**`ak_init` is explicit because three things it does cannot be done later, and
two of them are one-shot per process.**

- It **installs rustls's crypto provider by name** rather than reading whichever
  one happened to be installed. One line, and it matters most in a worker
  container that loads customer code.
- It **installs the tracing and log bridges**, unless the host passes
  `AK_INIT_OWN_LOGGING`. `tracing::set_global_default` and `log::set_logger` are
  both one-shot per process, so who owns them is decided here or not at all; this
  is what open decision 7 in the base design was about.
- It **installs the panic hook** and checks the build id, because two copies of
  the staticlib in one process either share Rust's globals or split-brain them
  with no warning.

It is **idempotent under identical options**: a second call with the same options
returns `AK_ALREADY_INITIALIZED`, which is a success. A second call with
different options fails, because the one-shot installs cannot be redone. **There
is no `ak_shutdown`**, for the same reason: the process-global installs cannot be
undone, and every resource that can be released has its own destroy.

`abi_version` is passed in rather than only exported, so the check is made by the
side that knows what it was generated against, once, at the only point where
failing is cheap.

**Every other entry point requires `ak_init` to have returned successfully**, the
codec included, and returns `AK_ERR_UNINITIALIZED` if it has not. A host that
uses only the codec still calls it.

**Configuration precedence is fixed here and must be reproduced**: an explicit
setter beats the environment, the environment beats the JSON handed to
`ak_context_new`, and that beats the defaults. Inverting it turns
`GrpcClient__AllowUnsafeConnection`, a pod-spec environment variable, into a
silent certificate-verification bypass for a context that explicitly pinned a CA.

**`worker_threads` comes from `ak_runtime_opts` with a small explicit default,
never from `Runtime::new()`.** Rust reads the cgroup quota, so `cpu: 500m` rounds
down to one worker while a requests-only pod takes every CPU on the node
(measured: 23 threads, 1.49 GB of virtual address space, idle). Two workers to
four on two vCPUs cost 2 percent of throughput and doubled to quadrupled the
p999, so a client that silently takes a worker per CPU does not look slow, it
looks erratic.

**Ownership between handles is internal.** A call holds an `Arc` on its client's
storage, invisible to the ABI, so `ak_client_destroy` drops the host's reference
rather than pulling the ground out from under an in-flight call. One clone per
call creation, not per operation.

## 4. Common vocabulary

```c
/* Encode: a string or bytes field as DATA inside the group, never a call.
   `len` counts SOURCE code units, never bytes and never characters.
   `data` must stay valid for the duration of the codec call.
   `tc == NULL` means the field is absent. */
struct ak_str { const void *data; size_t len; ak_transcode_fn tc; };

/* Decode: an OFFSET into the buffer the host handed in, plus a byte length.
   8 bytes where a pointer pair was 24, and still meaningful after a JNI host
   has released a critical section, which is what lets that host build a String.
   `coder` is an optional host hint (see 7.4). */
struct ak_span { uint32_t off, len; uint32_t coder; };

/* The transcoder writes the host's own string representation as UTF-8 into the
   codec's buffer. `cap` is what is available at `dst` right now, which is
   normally the whole remaining buffer rather than a per-string reservation, so
   a grow is the exception and not the rhythm. If it needs more it asks; `grow`
   may move the buffer, which is why it writes back through pointers. Returns
   bytes written, or a negative ak error code. */
typedef int32_t (*ak_grow_fn)(void *sink, int32_t want, uint8_t **dst, int32_t *cap);
typedef int32_t (*ak_transcode_fn)(const void *src, size_t len,
                                   uint8_t *dst, int32_t cap,
                                   ak_grow_fn grow, void *sink);

ak_transcode_fn ak_tc_utf8(void);    /* Rust String, Go string */
ak_transcode_fn ak_tc_utf16(void);   /* .NET string, JVM String */
ak_transcode_fn ak_tc_latin1(void);  /* JVM compact form, CPython 1-byte */
ak_transcode_fn ak_tc_ucs4(void);    /* CPython 4-byte */
ak_transcode_fn ak_tc_bytes(void);   /* memcpy, no validation */
```

**Why the transcoder is data rather than a callback.** There are not many
representations a host actually holds, so the core implements all of them once
for every language, and every managed host stops maintaining a UTF-8 encoder.
This is the largest single change from the drafted interface and it removes
machinery rather than adding it: no write callback, no capacity slot, no clamp
and no half-open rollback anywhere in the ABI.

**How many bytes an input can produce is the transcoder's business, not the
ABI's.** An earlier draft had the transcoder declare `max_bytes_per_unit` so the
codec could size a reservation. That is gone, and what it buys is worth stating
because it is not only simplification:

- **The under-declared bound is gone as a failure mode.** A declared bound is a
  correctness contract a host can be wrong about, and being wrong about it was
  measured as *silent wire corruption*: a transcoder that doubled every byte and
  declared 1 returned success and wrote a zero-length prefix followed by zeros.
  With no declaration there is nothing to under-declare.
- **The expansion table leaves the specification.** Whether UTF-8 passthrough
  declares 1 or 3 was coupled to whether it fails or substitutes on malformed
  input; that coupling is gone and the question is now purely about semantics
  (open decision 3).
- **The measured win for expressing the bound per code unit rather than as a
  ratio survives as an argument for `len` being in code units**, which it still
  is. The 3.0 to 14.1 percent that form was worth came from avoiding a hardware
  divide per string; no form of it remains in the call path.
- **What it costs is one check that cannot be removed**: the codec refuses a
  returned count larger than the capacity it gave, because nothing can make a
  transcoder that writes past its buffer safe. A transcoder that over-runs and
  does not ask is refused with `AK_ERR_CAPACITY` and none of its field is
  written.
- **It is not free of risk, and the slices measure it rather than assume.** The
  codec no longer knows what to reserve, so the grow path is exercised by
  whatever the buffer has left. The first build of the callback form produced
  messages exactly one byte short, because a length prefix sized from a stale
  reservation had to shift right into room a grow had not left. Prefix width is
  therefore always resolved after the transcode returns (section 6), and a
  payload built to cross a varint boundary (P2.4) is in the corpus for it.

## 5. Errors

**An error channel exists from the start, on every path, and it lives in the
context.** This is not a late addition to be retrofitted: an accessor that cannot
fail is a process abort, because a managed exception inside a reverse call does
not propagate and cannot be caught, and the generated guard that catches it needs
somewhere to put what it caught.

```c
typedef struct { int32_t code; uint32_t msg_len; const char *msg; } ak_err;

/* Any host code holding a context may fail the operation. Sticky: the first
   error wins, so unwinding cannot overwrite the cause. Never allocates, never
   throws, safe from inside a reverse-call frame. */
void    ak_fail(void *ctx, int32_t code, const char *msg, uint32_t msg_len);
int32_t ak_ctx_err(const void *ctx, ak_err *out);   /* AK_OK if none */
```

Three rules make that uniform rather than a per-shape arrangement.

- **Every host-facing callback takes the context as its first argument.** The
  loop callbacks, the decode `apply`, the batched `add`, the element maker. A
  callback with no context would be a place where a failure has nowhere to go,
  and one register is a cheaper price than a second error convention.
- **The codec checks the sticky slot after every upcall** and unwinds: it rolls
  the field and the message back to positions it recorded, stops, and the entry
  point returns the code. The output buffer is left for the host to discard or
  `ak_enc_reset`.
- **The generator emits the guard.** Every reverse-call accessor in a managed
  binding is wrapped so that an exception becomes `ak_fail` plus a return, and it
  is generated rather than left to a binding author's discipline.

On encode the root group is filled by the host *before* it calls in, so a failure
there needs no channel: the host simply does not call.

```c
#define AK_OK                   0
#define AK_ALREADY_INITIALIZED  1   /* success */
#define AK_ERR_HOST            -1   /* the host reported through ak_fail */
#define AK_ERR_MALFORMED       -2   /* invalid wire */
#define AK_ERR_TRUNCATED       -3
#define AK_ERR_DEPTH           -4   /* decode recursion limit */
#define AK_ERR_LIMIT           -5   /* message size limit */
#define AK_ERR_TRANSCODE       -6   /* the transcoder refused its input */
#define AK_ERR_CAPACITY        -7   /* a transcoder wrote past the capacity given */
#define AK_ERR_INVALID_STATE   -8   /* e.g. two concurrent recv on one call */
#define AK_ERR_PANIC           -9   /* a caught Rust panic, with its message */
#define AK_ERR_UNINITIALIZED  -10   /* ak_init was not called */
#define AK_ERR_ABI            -11   /* version or group-layout mismatch */
```

**Every entry point carries an error path, including the void ones**, because a
Rust panic crossing `extern "C"` aborts the process: `catch_unwind` is mandatory
everywhere, and a caught panic needs somewhere to put its message. There is no
thread-local `ak_last_error()`; the context is the place, and an entry point with
no context takes an `ak_err` out-parameter.

**Cost, stated so a slice does not inherit an optimistic margin.** The guard
measured +1.1 ns on a scalar accessor and +2.9 ns on a string accessor, and every
published figure in both managed reports was measured *without* it. v1 makes that
cheaper than it was rather than free: with strings riding in the group, the
accessors that remain are one per repeated field and one per element rather than
one per string, so the guard lands on tens of calls per message instead of
thousands. **Every slice measures with the guard on.**

## 6. Encode

**The group carries the whole singular subtree, unconditionally.** One struct per
message holding every scalar, every `ak_str`, and every singular child inlined
into it, with a presence word. A child is inlined whatever its size: a group
needs a fixed *shape*, not a size bound, and a string inside an inlined child is
an `ak_str` exactly as the parent's own are. A repeated or map field inside an
inlined child keeps its loop slot and is reached through the parent.

```c
/* One vtable per message, holding only what could not ride in the group. A
   message with no repeated field has an EMPTY vtable. */
struct ak_evt_TaskDetailed {
  ak_loop_blob_f loop_parent_task_ids, loop_data_dependencies;
  ak_loop_blob_f loop_expected_output_ids, loop_retry_of_ids;
  ak_loop_pair_f loop_options_options;          /* the map: a repeated pair */
};

intptr_t ak_encode_TaskDetailed(const void *obj, ak_enc_ctx *ctx,
                                const struct ak_evt_TaskDetailed *vt,
                                const struct ak_efix_TaskDetailed *fix);

/* What the host calls are PLAIN EXPORTS, not a table. */
int32_t ak_str_elem(void *ctx, const void *data, size_t len,
                    ak_transcode_fn tc);                   /* declines, never allocates */
int32_t ak_blob_reserve(void *ctx, int32_t want);          /* may allocate, may fail */
int32_t ak_run_i64(void *ctx, const int64_t *p, size_t n); /* one per host layout */
int32_t ak_elem_ResultRaw (void *ctx, const struct ak_efix_ResultRaw *elems, int32_t n);
int32_t ak_elemu_TaskDetailed(void *ctx, const struct ak_efix_TaskDetailed *elems,
                              int32_t n, int32_t tok0);
```

| field shape | how it crosses | crossings |
|---|---|---|
| singular scalar, and every scalar of an inlined child | rides in the group | 0 |
| singular string or bytes | rides in the group as `ak_str` | 0 |
| packed repeated scalar | the host's own array, handed over whole | 1 per field |
| repeated string or bytes | element call, batchable | N+1, or 1 per chunk |
| repeated message, leaf element (maps included) | element call, group filled by the host | N+1, or 1 per chunk |
| repeated message, non-leaf element | the same, plus a token per element | N+1, or 1 per chunk |

**A oneof is a discriminant plus every member inlined flat, not a union.** The
discriminant carries the active member's tag. A union would make the group's
layout depend on which member is largest, and section 10 requires group layouts to
be exported and asserted precisely because a host that reproduces offsets by hand
(FFM, any manual-layout binding) can get them silently wrong; a layout that also
depends on the widest member is a worse thing to reproduce. The cost is group
size, five slots where one would do for `Probe`, and it is paid on a shape the
schema has 19 of. **The union is an unmeasured alternative rather than an
equivalent**: nothing has priced it, and a slice that wants it priced adds an arm.

**Measured, a oneof and an explicit-presence scalar each cost zero crossings**:
both ride in the group entirely, 3 crossings for 200 elements in both directions.
An explicit field's encode branches on the presence bit and never on the value or
the length, which is what lets a present-and-empty string be written as present.
That is the mechanism the .NET gap left untested, and it is the part for a managed
binding to copy, rather than the counts.

**One element entry point taking a count, not two symbols.** `n = 1` is the
unbatched call. One codec body serves both shapes at no measurable cost and in
less compiled code, and the alternative is a second protocol with its own
rollback semantics to specify and test. The host may switch forms per field and
per element mid-stream, because every entry point appends: tested by feeding one
repeated field through three entry points in one pass, one element at a time,
then eight, then alternating on a runtime predicate, byte-identical throughout.

**Batched element runs are host-driven and chunked at 32 KB.** The host fills an
array of element groups from objects it is already walking and hands over
extracted data; the codec never names a host object, never dereferences one and
nothing is pinned. The leaf form (`ak_elem_X`, element type transitively free of
repeated and map fields) is the default because its ownership story is simplest:
the codec makes no reverse call during a run, so the entry point can be declared
non-suspending and a host exception cannot happen mid-run. The unrestricted form
(`ak_elemu_X`) names element *i* as `tok0 + i` from a contiguous token range the
host allocated, costs one more bounded 32 KB buffer per nesting level, and is
about 4 percent better on nested payloads. That is a trade, and the leaf form is
the default.

Its value differs by runtime by construction, which is why both slices are right:
it is worth 2 to 9 percent on JNI (a forward call is 11.2 ns), about half that on
FFM (3.4 ns), and nothing measurable on .NET (about 1.5 ns). **A host may decline
to batch at all** and loses only what its own crossing costs.

**A packed repeated scalar is the host's own array, handed over whole.** One
symbol per host layout; the wire encoding comes from the schema and lives in the
context, so `bool` and `enum` need no cases. **"Its own array" is doing work in
that sentence**, and the Rust slice measured where: a `Vec<TaskStatus>` is not the
wire representation, so the binding materialises a contiguous array first and part
of the crossing saved is paid back as a copy. A host that already stores the wire
form hands over a pointer and copies nothing; a host that stores an enum type does
not. The cost is the host's to avoid by choosing its storage, and the
specification implied it rather than stating it. A host with no contiguous layout
emits runs of length one and loses nothing.

**Length placeholders use a learned width, held in the encode context.** The
descriptor proves which messages can never exceed a one-byte length, and the
generator emits a form with no branch and no move for those. For the rest the
encoder reserves its best guess and moves only on a miss. **The table lives in
the context, never process-global**: 37 live slots at four bytes pack about
sixteen to a cache line, and a global table made two encoding threads slower than
one. Do not pad the prefix to a fixed width: it was built three ways and refused
three ways, most sharply because padding to the learned width makes the encoder's
output depend on its own history, which a byte-vector corpus cannot express and
which lets two threads of one process emit two different legal encodings of the
same message.

**Nothing the host calls in the codec is a table.** The host links the codec, so
it knows the symbol; a table adds an indirection, a layout that has to be
versioned, and a failure mode where a newer host reads a slot an older codec
never wrote. A missing symbol is a load failure, which is loud. Measured worth
nothing either way, so this is an interface-size and failure-mode argument and
should be made on those grounds. **The other direction cannot have that**, and
the asymmetry is forced: a managed method has no symbol, and a function pointer
is the only callable address .NET, the JVM and CPython can produce.

**An element or run entry point must leave the codec's open-field state as it
found it.** The host may call one more than once per field and the codec does not
get control in between, so an entry point that reads the open tag and site from
the context at entry, and lets the element body overwrite them, writes every
chunk after the first **under the inner field's tag**. That is silent wire
corruption, it was built and found in the Rust slice, and **byte identity did not
catch it**: the outer repeated field and the inner map field were both tag 1, and
no message in `SHAPES.md` distinguishes them. Save at entry, restore before
return. The hazard is not Rust's; any implementation holding "which field is
open" in the context has it, which rule 2 makes the natural design.

**Encode does not get the batching predicate's saving, and the asymmetry is
structural.** Decode costs **7.004 crossings per `TaskDetailed`**, measured, which
is exactly what 7.2 predicts against the drafted ABI's 43. Encode on the same
payload costs **10.02 per element**: five reverse calls, one per loop slot, and
five forward calls, four blob runs and one pair run. On decode the codec owns the
buffer and can defer; on encode the host drives every one of its own containers,
so a loop slot is a crossing whatever the predicate says.

**Every loop and every element call returns `int32_t`**, and a host that fails
mid-iteration says so through `ak_fail` on the context it was handed (section 5).
The codec rolls the field and the message back to positions it recorded. This was
the widest hole in the drafted interface, because a loop can fail after writing
half a field.

**The fill must be total.** Every scalar, every count and all three words of
every `ak_str` are assigned unconditionally, and the presence word is assigned
rather than OR-ed. In exchange the codec does not reset the element group between
elements, worth 5.4 ns per `ResultRaw` and 24.4 per `TaskDetailed`. This is an
invariant and it belongs in the header: a partial fill does not fail, it silently
inherits the previous element's value.

**What the total fill costs is the absent path**, measured for the first time by
the Rust slice and carried as open decision 9: the fill is unconditional, so on a
payload whose elements encode to nothing there is nothing for it to amortise
against, and the group turns a win in both directions into a loss in both. Do not
quote the group's worth from a full payload alone.

## 7. Decode

**Decode needs less machinery than encode, not more**, because the codec already
owns the bytes: the span points into the buffer the host handed in, so there is
nothing to reserve, size or transcode.

**And decode converges to parity with the incumbent in proportion to how much
host-side container construction an element needs**, which bounds what any of this
machinery can be worth. Three shapes measured in the Rust slice, `core-native`
against prost: a flat 5-field message with one or two strings decodes at 0.81 to
0.82; a 10-field message with six blobs and two optional children at 0.83 to 0.86;
a 27-field message with four `Vec<String>`, a `BTreeMap` and a nested child at 0.89
to 0.96. Strings alone do not explain it, since the first two allocate plenty; a
map insert and four vector growths are work every arm does identically and no
codec can avoid. Measured in
the Rust slice on P2.2, the shape the control plane actually moves: 17,500 strings
and 2,000 map entries in 551 KB, where both core arms land at parity with prost
(0.81 to 1.21) against 0.75 to 0.89 on the flat M1 payloads. The crossings are not
the reason, and this is what makes the finding portable: 7 per element at 1.8 ns
is 12.6 ns against ~2,200 ns per element, 0.6 percent, and the **no-boundary**
control is at parity too. What dominates is `String` allocation and map insertion,
which every arm does identically. The interface cost is still there and still
small, about 6 percent of decode on M2 and 2 percent on M1.

Two things follow for the slices. **A decode win measured on a payload less
string-dense than P2.2 may not survive P2.2**, so every slice reports it, and a
managed slice whose published decode figures came from thinner payloads should
expect them to shrink. And **the remaining saving on decode is allocation, not
crossings**, which is what decision 10 is about.

**The core does not transcode on decode, and the asymmetry has a reason.** Each
side transcodes into the memory it owns. On encode the destination is the codec's
buffer. On decode the destination is a `System.String` or a `java.lang.String`,
which only the runtime can allocate and which must be exactly sized at
allocation, so the host writes and its writer is the platform's fused intrinsic.
Consuming UTF-8 has no per-character loop to take over, and measured, the
transcoder on decode is a wash (0.86 to 1.08).

**That creates the one policy the ABI must state rather than leave to defaults:
malformed input.** Both halves replace it with U+FFFD. This is not cosmetic:
protobuf-java writes `?` for an unpaired surrogate and Google.Protobuf writes
U+FFFD, today, silently, and one shared transcoder cannot reproduce both. Moving
the transcoder into the core changes the observable bytes of at least one host
for input that was never valid, and that is a migration note rather than a defect.

### 7.1 Two delivery families, one traversal emitter

This is where the C# and Java findings disagree, and the disagreement is real
rather than a measurement artifact.

| Family | Who drives | Shape | Best for |
|---|---|---|---|
| **push** | the codec | the codec fills a bounded arena and calls `apply` / `add_<field>(obj, elems, n)` | a host whose reverse call is cheap: C++, and .NET |
| **pull** | the host | `parse` into a host-owned context making zero upcalls, then `drain` in 32 KB chunks through forward calls, then walk heap arrays | a host whose reverse call is dear: the JVM (33.8 ns FFM, 98.4 JNI), and probably CPython |

The pull family exists because on the JVM a push entry point makes upcalls, and
upcalls and a critical section are mutually exclusive, so pushing forces the wire
buffer to be copied into native scratch first. The Java slice built the push form
(as `BDP`) and measured it 1 to 14 percent slower than pull, the whole deficit
being that copy. The C# slice measured the pull form's intermediate as costing an
estimated 12 to 19 percent of a parse on a runtime where the crossing it saves is
worth 10 ns.

**So the ABI carries both, and a binding chooses.** The condition that keeps this
from being a fork: **one traversal emitter parameterised by where values are
deposited**, not two emitters that have to agree. Two emitters is a fork at the
generator level and it is the most likely place for the two families to drift
apart on a shape nobody tested.

```c
/* push: one entry point per message, the arena is a local of THIS function, so
   it is per decode rather than per thread: reentrant and allocation-free. */
int32_t ak_decode_ListResultsResponse(ak_dec_ctx*, void *obj,
                                      const uint8_t *buf, size_t len,
                                      const struct ak_dvt_ListResultsResponse *vt);
struct ak_dvt_ListResultsResponse {
  void (*apply)      (ak_dec_ctx*, void *obj, const struct ak_dfix_ListResultsResponse *fx);
  void (*add_results)(ak_dec_ctx*, void *obj, const struct ak_dfix_ResultRaw *elems, int32_t n);
};

/* pull: the same host-owned ak_dec_ctx of section 3, so the host can hold two
   decoded responses, read what it is paying, bound it and release it. Parse and
   drain cannot be one call, because a critical section and an upcall are
   mutually exclusive on the JVM. */
int32_t ak_bdr_reserve  (ak_dec_ctx*, size_t bytes);
size_t  ak_bdr_footprint(const ak_dec_ctx*);
int32_t ak_parse_ListResultsResponse(ak_dec_ctx*, const uint8_t *buf, size_t len);
int32_t ak_bdr_drain    (ak_dec_ctx*, uint8_t *dst, int32_t cap, size_t *cursor);
```

### 7.2 Batching predicate, and why batches never nest

**A repeated field may be handed over as a run if and only if its element type
contains no repeated and no map field, transitively.** Batching defers the host
call to the end of the field, and a repeated field nested inside an element needs
the host to have made that element first: there is nothing to attach the inner
elements to. The predicate is computed from the descriptor, and it is the same
predicate the encode side uses to decide what rides in the group, widened to
allow strings.

Its consequence is worth stating separately: **batches never nest**, so one arena
per context is enough. `ListResultsResponse.results` qualifies and a thousand
rows arrive in three calls; `ListTasksDetailedResponse.tasks` does not and keeps
two calls per element, which is 7 crossings per task where the drafted ABI spent
43.

### 7.3 The arena, and the flush that makes it correct

The run is materialised in a fixed-size arena sized as **a byte budget divided by
the group size**, not an element count, so the scratch is the same 32 KB whatever
the schema does. Bounding at 32 KB measured free (0.999 to 1.013) because the
destination stays in L2 however large the message is.

**The arena is flushed whenever a tag arrives that does not belong to the open
batch.** This is correctness, not tidiness: protobuf permits a repeated field's
occurrences to be interleaved with other fields', so `A B A B` is legal wire.
prost and Google.Protobuf writing each field contiguously is a property of those
writers, not of the format. With the flush, contiguous input batches fully and
interleaved input degrades to the unbatched cost. Tested by splicing a foreign
tag between every one of a thousand elements.

**The arena is never a thread-local.** A thread-local is a hidden global with a
re-entrancy hazard; the push family declares it at the top of the entry point
(per decode, reentrant by construction) and the pull family puts it in the
host-owned context.

### 7.4 Two rules for a facade author

**Resolve spans against the base pointer you already hold.** The host pinned the
buffer to make the call, so an offset is one add and then the same fused
transcode. Indexing the managed array instead measures 14 percent worse, and it
is the obvious thing to write.

**A batched add may be called more than once per field.** Append; never size to
the count you were handed.

`ak_span.coder` is a host hint (whether the bytes are Latin-1, so a JVM host can
take a straight compact copy). It is a host-specific field in a shared struct and
is **optional in this specification**: see open decision 4.

## 8. Bulk bytes: the direct-argument path

A small range of `ak_str.data` values is reserved as sentinels meaning *this
field is a direct argument of the call* rather than a pointer into staging. It is
one sentence in the specification and the one unambiguous win on the JVM: a
multi-megabyte result upload at 0.16 to 0.34 of protobuf-java, against roughly
parity for a staged path. `critical(true)` on FFM, `GetPrimitiveArrayCritical` on
JNI; C++ and C# never use it and pay nothing for it.

It generalises untested: it is built for one field of one root message and
nothing tests it on a nested message. **The generator-time refusal now exists**,
built by the Rust slice and run as a build step: it rejects a direct field on a
message tree that also needs a reverse call (a critical section and an upcall are
mutually exclusive, so it is a contract no host can honour) and more than one
direct field in one tree. Eleven lines, a predicate over the descriptor computed
where every other predicate is, so the cost of the rule is not an argument against
it.

**Rust cannot confirm what this path buys, and the reason is worth stating.**
There is no pinning to avoid and a copy is a copy either way, so the slice
measures it byte-identical and says nothing about the 0.16-to-0.34 figure above,
which is a JVM number. Worse for that figure: on a 4 MB decode the slice's
*no-boundary* control sits on the raw `memcpy` floor, so what the bulk path beats
there is the incumbent's copy strategy rather than a boundary cost. The
direct-argument path's value remains a JVM claim resting on one slice.

## 9. The RPC half

Its lifecycle is section 3. Unchanged in shape from the base design, and it is
the half whose case is
behavioural rather than performance: one retry set, one backoff, one TLS
configuration, one cancellation contract, enforced rather than copied.

**Two crossings per call, zero per field, and it is a property of the code rather
than a measurement.** The Rust slice built it and neither its RPC crate nor the
core's RPC module mentions a message type anywhere: the half dispatches on a path
string and moves opaque bytes, so there is no place a per-field cost could enter.
This is what the "adopt the RPC layer, generate the codec" fallback rests on, and
it is now checkable by reading two files rather than by trusting this paragraph.

**What that is worth, in a form that does not depend on the host.** Two crossings
of 1.8 ns against a call of about 1.5 ms of CPU is roughly four parts in a
million. A host whose crossing costs 98 ns through JNI pays 196 ns on the same
call: 0.013 percent. Measured end to end the Rust arm is 0.91 to 1.10 of tonic at
1, 8 and 16 in flight, which on four shared vCPUs is no measurable difference
rather than a win. **The arithmetic is the transferable part; the ratio is not.**

**A client handle is usable from many threads at once**, which "ownership between
handles is internal" in section 3 implies and which is easy to build wrongly: a
call must take a shared reference to its client and clone the cheap transport
handle, not take a mutable one. Built the other way first in the Rust slice, it
worked at 1 call in flight and failed outright at 8, which is the good failure
mode; the bad one is a wrong byte under contention. That it was found by a
throughput arm asking for 8 in flight, rather than by the concurrency suite of
obligation 12.5, is an argument for that suite rather than against it.

```c
ak_status ak_call_unary   (ak_client*, ak_bytes_in path, ak_bytes_in req,
                           ak_call_opts*, ak_bytes *out, ak_call **handle, ak_err*);
ak_call  *ak_call_unary_cb(ak_client*, ak_bytes_in path, ak_bytes_in req,
                           ak_call_opts*, ak_completion cb, void *user_data);
ak_call  *ak_call_unary_q (ak_client*, ak_bytes_in path, ak_bytes_in req,
                           ak_call_opts*, ak_queue*, uint64_t tag);
ak_call  *ak_call_open(ak_client*, ak_bytes_in path, ak_call_kind, ak_call_opts*);
ak_status ak_call_send(ak_call*, ak_bytes_in, bool last, ak_err*);
ak_status ak_call_recv(ak_call*, ak_bytes *out, ak_err*);
void      ak_call_close(ak_call*);    /* cancels, unblocks a pending recv, does NOT free */
void      ak_call_destroy(ak_call*);  /* frees, only after every operation returned */
```

Four amendments to the base design, all from the Java finding, all closing gaps
rather than changing shape:

- **The blocking call takes a handle.** Without one it cannot be cancelled, so a
  shutting-down application waits out every in-flight call.
- **Metadata as a key and value on the call**, a **deadline in milliseconds**
  that becomes `grpc-timeout`, and **the gRPC status code as its own number**
  beside its message. The third is not a convenience: ArmoniK's retry policy is a
  function of the status code, so a host that cannot tell `NOT_FOUND` from
  `PERMISSION_DENIED` cannot implement it.
- **Two delivery modes, and the queue ships with one drainer.** A callback is an
  upcall onto a thread the host does not own, which suits C++ and C#. A
  completion queue is a downcall the host blocks in, correlated by `uint64_t`
  tag so managed hosts need no pinning. The queue is a callback pushing onto a
  channel, so it is strictly additive. Its case is not amortisation (the drain
  ratio never exceeds 2.19): a thread parked in a drain is in native state and
  costs a collection nothing, and on virtual threads it is the fastest arm.
- **At least one mode in which the caller waits in the host language.** Blocking
  in a native frame from a virtual thread pins its carrier; what fixes that is
  parking in Java on a future, which the callback mode already provides. The
  requirement on the ABI is this weak and this general.

**Not offered: the host executor slot.** Built and measured on two runtimes,
earns its complexity on neither. Two structural findings from building it are
kept: it needs a bootstrap drainer, because hyper spawns the connection task
through it and `connect()` cannot complete until something drains; and
`ak_runtime_destroy` must not run while a host thread might be inside a poll,
because a thread inside a native call does not observe an interrupt. **Offered
instead**: a current-thread runtime, described accurately as "no worker pool, one
mostly-parked thread" rather than "shares the host's threads".

**The streaming concurrency contract** is unchanged from the base design:
`send || recv` on one call allowed from any threads, `send || send` and
`recv || recv` refused with `AK_INVALID_STATE` through a per-direction atomic and
a try-lock, `close || anything` allowed and must unblock, `destroy || anything`
forbidden. Under callback delivery, "returned" means the completion has fired.

## 10. Lifetime, versioning and load-time checks

- **Every handle is a plain pointer**, created and destroyed explicitly by the
  host, which is how every C library works and what RAII, `SafeHandle` and an FFM
  `Arena` all expect.
- **`close` and `destroy` are separate operations** and collapsing them is not
  merely risky but unimplementable: a woken `recv` re-acquires the mutex inside
  the object to finish waking, which TSan catches as a use-after-free with no
  ordering of stores that avoids it.
- **One `ak_abi_version()`, checked once in `ak_init` (section 3).** The whole
  ABI versions as a unit; not a size field per table. The host-called direction
  needs no version, because a mismatch there is a link failure.
- **Group layouts are exported and asserted at load.** A disagreement between
  `#[repr(C)]` and a host's layout otherwise surfaces as a wrong value in a
  field, which is the worst way to find it. Four lines, and it is insurance for
  the hosts that reproduce offsets by hand (FFM, and any manual-layout binding);
  a generated C shim taking the layout from the same header cannot restate it
  wrongly.
- **A token is an index, never an address**, and the codec never dereferences
  one. Document it, because a host that assumes otherwise builds a pinning scheme
  it does not need.
- **Document whether `ak_call_unary_cb` may invoke its callback before
  returning.** A host that finds out the hard way finds out as a re-entrant lock.

## 11. Deliberately not in the ABI

| Not in it | Why |
|---|---|
| a map case | a map is a repeated field of a pair message, and the repeated-message path handles it both ways. Specialising it into two strings is correct for one instantiation of one container |
| a scatter/gather decode | built and measured at 0.90 to 0.95, and it would give up the property that a span is an offset into one buffer, which is what the JVM needs. Renting the flatten buffer recovers most of it in three lines |
| a host executor slot | section 7 |
| a thread-local anywhere | a hidden global with a re-entrancy hazard; the context is the replacement |
| a tape, or any positional value stream | fastest measured encode arm on the JVM (0.99 to 1.16) and refused on architecture: a tape is a wire format, with a grammar to specify, version and debug across two languages, which is what protobuf already is |
| batched submission, call fusion | under one percent at best and not stable in sign |
| a runtime schema fingerprint | both sides ship from one release, so a disagreement is a codegen bug: it belongs in CI as a build-time subset check, not in a runtime guard |

## 12. Conformance obligations ABI v1 creates

1. **The byte corpus is a release gate**, generated from the descriptor, with
   unknown fields, absent fields and every field shape (README section 10).
2. **The transcode pair**: encode transcodes in the core, decode transcodes in
   the host, so the two must agree on malformed input across every facade. The
   corpus carries the pair, and the unpaired-surrogate substitution is named in
   this document rather than left to each platform.
3. **Group layout assertions at load**, per section 8.
4. **The worker path's build-time subset check.** On the client path the two
   sides need not agree about the schema at all, since opaque bytes cross. The
   worker path is the exception and it hosts customer code: Rust decodes
   `ProcessRequest` and encodes three request types, so two decoders read one
   buffer and a disagreement is silent. CI asserts that the fields Rust reads are
   a subset of what the facade encodes, for those five messages.
5. **A concurrency suite that runs at least two payload shapes**, and it is the
   obligation with the most evidence behind it and the least existence: the Rust
   slice found a shared-mutable-client defect (section 9) *by accident*, because
   stage 4 happened to ask for 8 calls in flight, and nothing in any slice looks
   for that class on purpose. Of at least one
   message type, with threads run in sequence as well as together, and every
   encode asserted against a reference rather than counted. A suite with one
   shape reports zero wrong bytes with a per-thread-state defect present and
   absent alike; two shapes find it in twenty encodes out of twenty.

## 13. Open decisions

Each blocks something. None is settled by a measurement that exists today.

1. **Is every mechanism free under the C++11 floor?** They were motivated by
   managed hosts. That C++ pays nothing for the group, the string-as-data form
   and the batching predicate is currently an argument. Settled by the C++ slice,
   or earlier if it is cheap to check. **Blocks: freezing this document.**
2. **Which decode family does each binding take** (7.1), and is the single
   parameterised emitter actually buildable? Settled by the first two slices that
   pick different families.
3. **Which UTF-8 validator?** It was framed as semantics (fail, or substitute;
   the Java slice chose fail), then as a cost (the Rust slice priced validation at
   25 to 30 percent of an encode on ASCII, and proposed trusting a host type that
   carries the invariant). **The content-set pass reframed it again, and this
   framing is the one to carry**: the choice that matters is not whether to
   validate but *which validator*, because most of what trusting was buying is
   available without giving up the contract.

   **The ASCII figure was not the cost.** `core::str::from_utf8` consumes a
   `usize` at a time on ASCII and one byte at a time otherwise, so its cost tracks
   **non-ASCII bytes, not bytes**. On the Latin-1 and above-U+00FF content sets the
   scalar validator costs 2.2 to 3.0 times its own ASCII cost and turns a 0.72 to
   0.81 win against prost into a **2.0 to 2.6 loss**: the largest single effect
   measured anywhere in this branch, and invisible to an ASCII-only pass.

   **A SIMD validator with the identical contract recovers half to two thirds of
   it**, to 1.27 to 1.50 of prost. Same refusal of malformed input, no trust
   extended to anyone, `simdutf8::basic` in place of the scalar DFA. It is not
   faster on ASCII, because the scalar ASCII path is already eight bytes an
   iteration. **The accept and reject sets are identical**, which is the condition
   that makes it safe here: a validator that differed on any input would make the
   core's output depend on which one a build chose, and that is the one thing this
   branch does not permit.

   **What survives of the earlier framings.** Trusting the host is still refused,
   and for the unchanged reason: it is a correctness contract a host can be wrong
   about, which is exactly what dropping `max_bytes_per_unit` removed, and being
   wrong about that one was measured as silent wire corruption. What changed is
   the price of refusing it: the gap between validating with SIMD and trusting is
   now 1.27 to 1.50 against 0.72 to 0.85, so trusting is still worth something and
   is no longer worth 2.6.

   **So the question put to the slices changes.** Not "does your host type carry
   the invariant" but **"what does your platform's UTF-8 validator cost on
   non-ASCII input, and is a faster one with the same contract available?"** For
   C++ that is a real choice, and simdjson's validator is the same family. For
   Python the incumbent is upb, which already validates in C, so the comparison
   may be nearly free, which would itself be informative.

   **Not settled, and none of it by this slice**: one x86-64 machine with AVX2;
   `simdutf8::basic` dispatches on runtime CPU features and falls back where they
   are absent, which makes it a **floor** question in C++ as much as in Rust; and
   it is a dependency inside the core rather than in a binding, so it carries a
   packaging cost nobody has priced. **Blocks: nothing; the default is unchanged
   and `ak_tc_utf8` remains what the specification names.**

4. **Is `ak_span.coder` in the shared struct or out?** It is a JVM-specific hint
   in a struct every language reads.
5. **What the grow path actually costs now that nothing is reserved from a
   declared bound** (section 4). The codec hands the transcoder whatever the
   buffer has left, so a grow should be rare, but no slice has measured the rate
   or what a grow costs when the length prefix has to be resized after it. The
   first slice to build the encode path answers it, and P2.4 is the payload for
   it. **This is the one decision created by v1 rather than inherited.**

   **Answered. Keep the learned width.** The Rust slice measured it per site,
   which an aggregate cannot do. On every uniform payload (P1.2, P2.2, P2.3,
   P2.5) a warm context misses **zero** times and moves **zero** bytes. On P2.4,
   built so that a per-site width is wrong on every element, it misses once per
   element and memmoves 980,938 bytes of a 981,222-byte output: the whole payload,
   once, every encode. Isolated against two size-matched uniform arms rather than
   attributed, and against prost as a floor for the construction's own
   non-linearity, **the mechanism costs 1 to 3 percentage points of an encode on
   the payload built to defeat it**, and nothing at all elsewhere. Each move is a
   sequential in-cache memmove of a ~12 KB element body. **Zero grow-callback
   invocations on any payload**, which is what handing the transcoder the whole
   remaining buffer was meant to buy.

   **The worst case cannot be engineered away, and that is the closing argument
   rather than a caveat.** Over-reserving needs a non-minimal varint, which
   section 6 refuses outright; under-reserving needs the move. The alternatives
   are a two-pass length computation (prost's) or writing each body to scratch
   first, and both cost every payload to spare P2.4. So the learned width is the
   right default at a bounded worst case, not a bet that the worst case is rare.
6. **The worker path.** Either Rust hands the facade the raw `ProcessRequest`
   bytes and the facade decodes them itself, which is two decoders over one
   buffer that can silently disagree, or Rust decodes once and exposes typed
   getters, which puts the schema back into the boundary and contradicts the
   fixed-size property everywhere else. Obligation 12.4 covers the first.
7. **Decode recursion limit.** Rust holds the reader and recurses in Rust, so
   prost's `RECURSION_LIMIT` does not apply and the built codec has none. The
   in-scope schema is acyclic with static depth 6 and unknown nested fields are
   skipped without recursing, so it is not reachable today and nothing enforces
   that. `AK_ERR_DEPTH` exists for it; the limit itself is unset.
8. **Message size limits.** `WorkerServer.h` sets `SetMaxReceiveMessageSize(-1)`
   today; tonic defaults to 4 MiB. Without an explicit per-direction knob,
   migration day gives `RESOURCE_EXHAUSTED` to every customer whose payloads
   exceed 4 MiB, which for an HPC orchestrator is normal. `AK_ERR_LIMIT` exists
   for it; the default does not.
9. **Does the by-value group need an empty-element path?** New, and the first
   measurement of a cost the amendment was known to have and had never been
   charged for. The group carries the whole singular subtree unconditionally
   (section 6), so the binding fills a ~200-byte element group and the core
   materialises a ~128-byte one **whatever the wire holds**. On P1.3, where every
   element encodes to nothing, that fixed cost has nothing to amortise against and
   **the verdict inverts**: `core-ffi-rust` goes to 1.16 to 1.39 of prost in both
   directions while the no-boundary control stays at 0.47 to 0.84
   (`ffi/logs/rust/stage2-four-arms-M1.log`). Every other M1 payload has the core
   ahead in both directions.

   This is not exotic input. A page of results where most fields are unset is
   ordinary control-plane traffic. The options are a presence-word fast path for
   an entirely empty element, letting an element run hand over a count of empties,
   or accepting the cost and saying so. **Nothing is decided on one slice and one
   message**: M2 (P2.5) is the nested absent case, and the managed slices pay a
   different price for the same fill because theirs crosses a runtime boundary. It
   is written down now so that no slice quotes the group's worth from a full
   payload alone. **Blocks: nothing; a candidate amendment, not a defect.**

10. **Can decode deliver the group before the runs?** The push family's two-call
   protocol (`new`, then `apply`) makes a host materialise a default element and
   then fill it, where the incumbent constructs it once. The order is forced as
   specified: runs may arrive before the group fields, so a binding that
   constructed from the group would discard them. The Rust slice observes that the
   codec **already** buffers runs in bounded arenas (7.3), so it could defer the
   flush to the end of an element body while the arena has room, allow `apply`
   first and one construction, and fall back to the current order when an arena
   fills. Nothing is built. It changes the decode contract, so it is written down
   here rather than tried in a slice. It is also the shape of saving that section
   7's measured note says is the one still available on decode: one construction
   per element is allocation, and allocation is what decode turns out to be.

11. **Does the core retain unknown fields?** Today it does not, and neither does
   prost, so nothing in this design carries an unrecognised field from decode to
   re-encode. **That is a behaviour change for four of the five languages.**
   proto3 has preserved unknown fields since protobuf 3.5, so
   `Google.Protobuf`, protobuf-java, protobuf C++ and upb all retain them and
   re-emit them; Rust is the one incumbent that already drops them. Adopting the
   core would therefore remove a protobuf guarantee from C#, Java, C++ and Python
   rather than from nobody.

   The Rust slice established the wire half of this rather than the policy half,
   and the wire half is not in dispute: an unrecognised tag cannot be
   distinguished from any other unknown field, so an unknown *oneof* member leaves
   the case at the last known member and the payload is dropped. Seven hand-built
   vectors, all four arms agreeing on the decoded value and the re-encoded bytes.
   What it did not price is what retention would cost, and the cost is not
   obviously small: a decode that keeps unknown bytes has to store them somewhere
   the host can hold, which is a host-visible allocation on a path the whole design
   works to keep allocation-free.

   **Who this bites is specific rather than general.** A client that decodes a
   response and never re-encodes it loses nothing. A proxy, a worker that forwards
   a `ProcessRequest`, or anything that round-trips a message between two versions
   of the schema loses the field silently. Obligation 12.4 already treats the
   worker path as the place two decoders meet over one buffer; this is the same
   seam seen from the other side.

   **Settled by**: deciding whether ArmoniK re-encodes anything it decoded, which
   is a question about the product rather than about the ABI, and then either
   pricing retention or writing the loss into the migration notes. **Blocks:
   nothing today. It is the largest unpriced behaviour change the branch has
   found, and it was found by a shape-coverage vector rather than by a benchmark.**

12. **The diagnostic contract**, and it is worse than "five failures render as
   one string". `ak_init` now owns the log and tracing bridges (section 3), which
   settles *who*. What is still open is *what*: five distinct transport failures
   currently render as one string, so `ak_err` needs a machine-readable failure
   class and the flattened source chain, and a host needs a restart-only
   transport-diagnostics dial. Today `GRPC_TRACE` is what an SRE reaches for at
   03:00 and there is no counterpart.

   **One level below that, the ABI has nowhere to put a cause at all.** The Rust
   slice's RPC path reported a real failure as `AK_ERR_HOST` with the cause
   discarded, because the call mapped its error away; that code was correct
   against this specification, which asks for a code and a message and provides no
   channel for a source chain. An error channel that discards the error is not an
   error channel, and the fix belongs to this decision rather than to a slice.

**Settled since the first draft**, kept here so a reader of an earlier version
does not look for them: the accessor error channel is now section 5 rather than a
gap, `max_bytes_per_unit` is gone rather than specified, and configuration
precedence is stated in section 3 rather than carried as a decision.

## 14. Provenance

Every amendment, what motivated it, and where the figure lives. A slice that
wants to revisit one starts here rather than re-deriving it.

| Amendment | From | The figure it turns on |
|---|---|---|
| by-value group carrying the singular subtree | C#, confirmed on JVM | decode 1.192 to 0.955 on P1.2; the JVM control without it is 1.22 to 1.64 times worse |
| string as data in the group (the triple) | C# and Java | the largest single change: 1.12 to 1.64 times on JVM encode, 25 to 36 percent on .NET |
| lengths in source code units | Java | the ratio-over-bytes form cost a hardware divide per string: 3.0 to 14.1 percent of an encode |
| transcoder growth callback, and no declared expansion bound at all | Java, then v1 | the callback closed silent wire corruption at 0.957 against the retry loop it replaced; v1 drops the declared bound with it, which no slice has measured (decision 5) |
| packed scalar as the host's own array | C# | 0.22 against 0.44 of `ToByteArray`; 2,001 crossings against 25,201 |
| one element entry point with a count | C# | no measurable cost, less compiled code, one protocol instead of two |
| host-driven batched element runs, leaf form default | Java | 2 to 9 percent on JNI, about half on FFM, nothing on .NET |
| batching predicate, transitive, from the descriptor | C# | 7 crossings per task against 43 |
| bounded arena in the context, flushed on a foreign tag | C# | correctness on legal interleaved wire, plus re-entrancy |
| decode spans as offsets into the host's buffer | C# and Java | what takes a 4 MB download from 4.2 times protobuf-java to 1.00 |
| pull-family drain | Java | decode 1.17 to 1.99 of protobuf-java becomes 0.68 to 0.92 |
| learned length-placeholder width, per context | Java | 1.32 to 2.23 times aggregate throughput at two threads, against a global table |
| direct arguments for bulk bytes | Java | 0.16 to 0.34 of protobuf-java on a 4 MB upload |
| plain exports rather than a table | C# | nothing measurable; interface size and failure mode |
| no map case | C# | tens of nanoseconds per entry, paid deliberately |
| group layout export and assert, one ABI version | C# and Java | insurance against the worst failure mode a by-value ABI adds |
| RPC: handle on the blocking call, metadata, deadline, status code | Java | a retry policy is a function of the status code |
| explicit `ak_init`, one-shot installs owned there | base design, v1 | the crypto provider, the log and tracing bridges and the panic hook cannot be installed late, and two of the three are one-shot per process |
| error channel in the context, guard generated | C#, then v1 | an unguarded accessor terminates the process; the guard costs +1.1 ns scalar and +2.9 ns string, and every published margin was measured without it |
| completion queue with one drainer; no executor slot | Java, C# | the queue is the best arm on virtual threads; the executor slot earns nothing on either runtime |
