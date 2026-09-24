# The C ABI, v1

**Status: draft for agreement (W1).** This is the specification every slice is
built against. Until it is agreed, no slice starts; once it is agreed, a slice
that disagrees with it raises a finding rather than diverging quietly.

Phase note (2026-09-24): container timings in this document were removed or
marked as instrumentation (design/FIX-PLAN.md WP2).

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
cost more in one direction than in the other, every measurement argues for moving one
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
(observed: 23 threads, 1.49 GB of virtual address space, idle). More workers
than vCPUs hurt tail latency far more than throughput (container
instrumentation; to be measured in the campaign), so a client that silently
takes a worker per CPU does not look slow, it looks erratic.

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

**The UTF-8 entry is a passthrough, and passthroughs do not validate.** Nothing
in the encoder needs a `string` field's bytes to be valid: it writes a length and
copies. The decoder cannot trust them whatever the encoder did, since they arrive
off a wire, and proto3 requires a parser to check. So `ak_tc_utf8` and
`ak_tc_bytes` are the same memcpy, and the check lives on decode where it is not
redundant. The other three entries convert, so they keep an explicit
malformed-input policy: see open decision 3.

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
- **The argument for expressing the bound per code unit rather than as a
  ratio survives as an argument for `len` being in code units**, which it still
  is. That form avoided a hardware divide per string; no form of it remains in
  the call path.
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
- **And it has a cost nobody listed, found by the C++ slice: the two-pass blob
  write.** Because the codec no longer knows the length in advance, every blob is
  written by opening a prefix of a learned width, handing the transcoder the rest
  of the buffer, and resolving the prefix afterwards. A host that *already holds
  the bytes* knows the length and could write key, length and body in one pass.
  On P1.2's 6,000 strings the two-pass write was slower per string than a
  one-pass write, and looked like a real share of the C ABI's encode gap against
  a no-boundary control on a string-dense message (container instrumentation; to
  be measured in the campaign).

  **The fix is a `tc == ak_tc_bytes` fast path in the core**: where the specified
  passthrough transcoder is in use, the codec may take `ak_str.len` as the byte
  length and write the prefix in one pass. It is free for every host whose
  representation is already UTF-8 (C++, Rust, Go) and changes nothing for a
  converting transcoder, which still cannot know its output length in advance. It
  is a core-side optimisation with no ABI surface, so it needs no host change and
  no version bump.

  **Worth reading beside upb, which takes the opposite route**: upb encodes
  *backwards* so that a length is always known by the time its prefix is written
  (`upb/wire/encode.c:8`, "We encode backwards, to avoid pre-computing lengths").
  That makes prefixes free and makes buffer growth expensive (`encode_growbuffer`
  memmoves everything written so far to the end of the new block), and the C++
  slice saw upb's encode slower than protobuf C++ on string-dense payloads
  (container instrumentation; to be measured in the campaign). protobuf C++ takes the third route, a full `ByteSizeLong`
  pre-pass and then an exact forward write. The learned width is a fourth point in
  that trade and decision 5 counted its misses at zero on every uniform
  payload, so the fast path above is a refinement of the design rather than a
  repair.

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
- **A reset clears the sticky slot, and an entry point never returns a stale
error.** `ak_enc_reset` and `ak_dec_reset` (section 3) clear it; so does the start
of a fresh top-level operation. Stated because the Rust slice built it the other
way and one rejected decode poisoned every later decode in that context, which is
the failure mode "the first error wins" produces if nothing ever says when the
slate is wiped.

**The codec checks the sticky slot after every upcall** and unwinds: it rolls
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

**That mandate is now measured rather than argued, and the core does not meet it.**
The rust slice's concurrency suite planted the obvious host misuse (four threads
sharing one encode context) as a positive control, expecting wrong bytes. It does
not produce wrong bytes: the core panics inside `Enc`, the frame the unwind must
cross is an `extern "C"` entry point, the unwind is refused, and **the process
aborts**. Section 3's panic hook changes what is printed, not whether that happens.
So every codec entry point is today exposed to the failure this paragraph exists to
forbid, and **`catch_unwind` being mandatory is specification that nothing enforces**.
Two things follow, and they are separate:

- **`catch_unwind` at every entry point is a conformance obligation, not a note**,
  and section 12 gains it with a test that plants a panic and requires
  `AK_ERR_PANIC` at the boundary. A codec that can abort its host process on a
  misuse the host is able to commit is not a drop-in for a library that throws.
- **`AK_ERR_PANIC` is the wrong diagnosis for this particular misuse and a better
  one is cheap.** An owning-thread id beside the context's existing `kind` word
  turns a concurrent use into `AK_ERR_INVALID_STATE` at the *first* misuse, before
  any state is corrupted, where a caught panic reports it afterwards and cannot say
  why. The cost is one word in the context and one comparison per entry point.

**Cost, stated so a slice does not inherit an optimistic margin.** The guard
has a per-accessor cost, larger on a string accessor than on a scalar one
(container instrumentation; to be measured in the campaign), and every
published figure in both managed reports was taken *without* it. v1 makes that
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
unbatched call. One codec body serves both shapes in less compiled code (its
run-time cost is container instrumentation; to be measured in the campaign), and
the alternative is a second protocol with its own
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
host allocated, costs one more bounded 32 KB buffer per nesting level, and was
faster on nested payloads (container instrumentation; to be measured in the
campaign). That is a trade, and the leaf form is the default.

Its value differs by runtime by construction: it saves forward crossings, and a
forward crossing costs different amounts on JNI, FFM and .NET, which is why the
two slices' observations need not conflict. **A host may decline to batch at
all.**

**What it then loses, or gains, is a function of its crossing price, and the C++
slice probed the curve rather than the point.** With a calibrated delay in front
of every forward entry-point call, the P2.2 delta between the unbatched and
batched arms changed sign as the added delay grew. **There is a crossover in the
forward crossing price.** Below it a host that declines to batch is *faster*,
because the chunk's second pass over memory costs more than the crossings it
saves; above it batching wins. Where the crossover lies, and which side of it
each host sits on, is container instrumentation and is to be measured in the
campaign.

So the statement is per host and not per specification. A slice that reports
only its own sign has not answered this; it reports its crossing price beside
it. (`findings/cpp.md`, `logs/cpp/tax.log`.)

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
encoder reserves its best guess and moves only on a miss.

**Two refusals live here and they are independent, which the earlier text ran
together.** A concurrency suite built for obligation 12.5 separated them
(`logs/cpp/concurrency.log`):

- **The table lives in the context, never process-global.** 37 live slots at four
  bytes pack about sixteen to a cache line. A global table is a data race and a
  **throughput** defect (slower when contended in C++ and in Java, and the
  slowdown appeared to track how often the table is *written* rather than the
  fact of sharing; container instrumentation, to be measured in the campaign),
  but it is **not a byte defect**: an unpadded prefix is rewritten to whatever
  width the body actually needs, whatever the guess was.
- **Do not pad the prefix to a fixed width.** Built three ways and refused three
  ways, most sharply because padding to the learned width makes the encoder's
  output depend on its own history, which a byte-vector corpus cannot express,
  and which lets two threads of one process emit two different legal encodings of
  one message.

**Only the combination corrupts, and it corrupts in a way a naive suite cannot
see**: the threads *agree* with each other, because they share the pollution, so
a suite that compares two threads' output finds nothing. It takes an independent
reference (the incumbent's encoder, not a re-encode with the code under test)
to catch it. Measured: one payload shape gives 0 wrong of 24, two shapes that
want different widths at a shared site give 44 of 48 (a count that includes each
wrong native encode twice, once directly and once through the round trip, so 22
distinct wrong encodes; and the planted builds did not reach the shared core,
FIX-PLAN R-D7).

**And the pair has to be chosen, not assumed.** Widths only ever grow within a
context, so only an ordered pair where the first shape leaves a site *wider* than
the second needs can reveal anything. Four shapes across two message types had no
such pair and the first suite passed every planted build. A suite for this
obligation asks the encoder which ordered pairs have a history surface and prints
the answer **even when it is empty**.

**Nothing the host calls in the codec is a table.** The host links the codec, so
it knows the symbol; a table adds an indirection, a layout that has to be
versioned, and a failure mode where a newer host reads a slot an older codec
never wrote. A missing symbol is a load failure, which is loud. Container
instrumentation showed no difference either way, so this is an interface-size
and failure-mode argument and should be made on those grounds. **The other direction cannot have that**, and
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
elements, which saves a reset per element (settled in the draft; timing evidence
is container instrumentation, re-checked in the campaign). This is an
invariant and it belongs in the header: a partial fill does not fail, it silently
inherits the previous element's value.

**What the total fill costs is the absent path**, measured first by the Rust slice
and carried as open decision 9: the fill is unconditional, so on a payload whose
elements encode to nothing there is nothing for it to amortise against, and the
group turned from a gain in both directions into a loss in both (container
instrumentation; to be measured in the campaign). Do not judge the group's
worth from a full payload alone.

**An alternative exists and it does not weaken this invariant.** If the
host bulk-clears its chunk buffer and then assigns only the fields that differ
from the default, the absent-path inversion disappeared in container
instrumentation with no visible cost on the other payloads (decision 9; to be
measured in the campaign). The codec still resets nothing between elements, which
is what this paragraph is actually buying; what changes is the host's side of the
contract, and a partial fill is safe only because the clear precedes it.

## 7. Decode

**Decode needs less machinery than encode, not more**, because the codec already
owns the bytes: the span points into the buffer the host handed in, so there is
nothing to reserve, size or transcode.

**And decode's distance from the incumbent appeared to shrink with how much
host-side container construction an element needs**, which would bound what any
of this machinery can be worth. The Rust slice compared three shapes,
`core-native` against prost: a flat 5-field message with one or two strings; a
10-field message with six blobs and two optional children; a 27-field message
with four `Vec<String>`, a `BTreeMap` and a nested child. The gap closed as
construction grew (container instrumentation; to be measured in the campaign).
Strings alone do not explain it, since the first two allocate plenty; a map
insert and four vector growths are work every arm does identically and no codec
can avoid. The same held on P2.2, the shape the control plane actually moves
(17,500 strings and 2,000 map entries in 551 KB), where both core arms came out
level with prost. The crossings are not the reason, and this is what makes the
finding portable: P2.2 decode makes 7 crossings per element (a count), against the
whole cost of building the element's host objects, and the **no-boundary** control behaved
the same way. What dominates is `String` allocation and map insertion, which
every arm does identically. The interface cost is still there and still small
relative to that construction (container instrumentation).

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
Consuming UTF-8 has no per-character loop to take over, and a transcoder on
decode showed no clear gain (container instrumentation; to be measured in the
campaign).

**That creates the one policy the ABI must state rather than leave to defaults:
malformed input.** With encode-side validation gone (decision 3), decode is
now the *only* place a `string` field's bytes are ever checked, so this policy
carries the whole of protobuf's UTF-8 guarantee rather than half of it. A facade
written the obvious way does not implement it: the Rust slice decodes through a
lossy conversion at 37 sites and rejects at none, so bad input silently becomes
U+FFFD. Whether the answer is reject or substitute, it is generated and asserted
rather than left to whichever call a facade author reached for. **Decode rejects**: proto3 requires a
parser to validate, `Google.Protobuf` and protobuf-java both throw, prost returns
an error, and decision 3 records rejecting as no dearer than substituting
(container instrumentation).
The U+FFFD-against-`?` divergence that motivated this paragraph is a *conversion*
question and stays with the converting transcoders on encode, where both halves
replace unrepresentable input with U+FFFD. This is not cosmetic:
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
| **pull** | the host | `parse` into a host-owned context making zero upcalls, then `drain` in 32 KB chunks through forward calls, then walk heap arrays | a host whose reverse call is dear: the JVM (FFM and JNI), and probably CPython |

The pull family exists because on the JVM a push entry point makes upcalls, and
upcalls and a critical section are mutually exclusive, so pushing forces the wire
buffer to be copied into native scratch first. The Java slice built the push form
(as `BDP`) and saw it slower than pull, the deficit attributed to that copy. The
C# slice estimated the pull form's intermediate as a real share of a parse on a
runtime where the crossing it saves is cheap. (Both container instrumentation;
to be measured in the campaign.)

**So the ABI carries both, and a binding chooses.** The condition that keeps this
from being a fork: **one traversal emitter parameterised by where values are
deposited**, not two emitters that have to agree. Two emitters is a fork at the
generator level and it is the most likely place for the two families to drift
apart on a shape nobody tested.

**Both families are now built in the shared core, from one emitter, and the
condition is met rather than hoped for.** `dec_walk` is emitted once and
instantiated twice; the families differ in a macro body, the entry point's
prologue and epilogue, and one argument naming the non-leaf element decoder. The
control that makes this checkable is structural rather than statistical: **pull
writes a record exactly where push makes a reverse call, so the two counts must be
equal**, and they are, to the digit, on all thirteen counted payloads (P2.2:
3,501 and 3,501), with pull's reverse count measured at zero everywhere. A host
gate on pull is by VALUE identity rather than byte identity, because a record
stream is not wire bytes; byte identity comes back when the drained values are
re-encoded. Open decision 2 carries what the families cost and how to re-price
them on a host whose reverse call is dear.

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
the schema does. Bounding at 32 KB keeps the destination in L2 however large the
message is, and it showed no cost in container instrumentation (to be measured
in the campaign).

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
transcode. Indexing the managed array instead was slower (container
instrumentation; to be measured in the campaign), and it is the obvious thing to
write.

**A batched add may be called more than once per field.** Append; never size to
the count you were handed.

`ak_span.coder` is a host hint (whether the bytes are Latin-1, so a JVM host can
take a straight compact copy). It is a host-specific field in a shared struct and
is **optional in this specification**: see open decision 4.

## 8. Bulk bytes: the direct-argument path

A small range of `ak_str.data` values is reserved as sentinels meaning *this
field is a direct argument of the call* rather than a pointer into staging. It is
one sentence in the specification and it is aimed at the JVM: a multi-megabyte
result upload skips the copy a staged path pays (its gain against protobuf-java
is container instrumentation; to be measured in the campaign). `critical(true)` on FFM, `GetPrimitiveArrayCritical` on
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
checks it byte-identical and says nothing about the JVM gain above. Worse for
that claim: on a 4 MB decode the slice's
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

**What that is worth, in a form that does not depend on the host.** The
per-call share of the boundary is two crossing prices divided by the CPU cost of
one RPC, whatever the message. Both terms are to be measured in the campaign; the
container figures that filled in this arithmetic, and the end-to-end comparison
with tonic, were instrumentation. **The arithmetic is the transferable part.**

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
  should cost a collection nothing, and on virtual threads it looked the best arm
  (container instrumentation; to be measured in the campaign).

  **BUILT, in the shared core, and this paragraph was specification with nothing
  under it until now.** `ak_call_unary_cb` and `ak_call_unary_q` are three
  deliveries of **one call path**, which is the condition 7.1 puts on the decode
  families for the same reason: two bodies that have to agree is a fork at the
  place nobody tests. The queue is `Mutex` plus `Condvar` rather than a receiver
  behind a lock, so a second drainer is slow rather than deadlocked.

  | delivery | forward | reverse | who blocks, and where |
  |---|---|---|---|
  | `ak_call_unary` | 2 (call, free) | 0 | a host thread, **inside the core** |
  | `ak_call_unary_cb` | 2 (call, free) | **1** (the completion) | nobody; the core calls out |
  | `ak_call_unary_q` | 3 (call, next, free) | **0** | a host thread, inside `ak_queue_next` |

  **The queue trades one reverse call for one forward call**, and that is its whole
  case on a host where the two are priced differently: on the JVM a cached upcall
  costs several times a forward crossing, so the trade favours the queue before
  the pinning question is even asked. On .NET, where the two directions cost
  about the same and the runtime has a future to complete from any thread, the
  callback is the natural one. (Crossing prices are container instrumentation;
  to be measured in the campaign.) **Neither is a default
  the ABI picks**, which is why both are exported.

  **Why this got built now, and it is an R14 finding pointed inward.** The java
  slice's transport arm was taken through the **blocking** mode, because it was the
  only one implemented, on the host whose own measurement (the fourth amendment
  below) says blocking in a native frame pins a virtual thread's carrier. A harness
  that makes the incumbent do extra work is a defect; so is one that makes the
  core's own arm take the delivery its host is worst at, and this was the second
  kind. The measurement stands as a blocking-mode measurement and is labelled one.

  **Observed on the JVM** (P2.2, JDK 17, client and server in one process, which
  is a known harness hazard; container instrumentation, to be measured in the
  campaign): the queue delivery used less CPU per RPC than the blocking delivery
  at 8 and 16 in flight. **At 1 in flight the queue used more** (submit then wait serialises
  what a blocking call does in one step, and pays a third crossing for it), which
  is the expected shape: the queue is a concurrency mechanism, not a faster call.
  On JDK 21 the same direction held at 16 in flight, and a virtual thread drained
  the queue with no visible penalty against a platform thread.

  **What that does NOT establish, because the slice said so rather than letting it
  pass**: it does not reproduce the carrier-pinning comparison. A queue has one
  drainer by design and one drainer needs one carrier either way, so this shows the
  queue is *usable* from a virtual thread, not that it *rescues* a host from the
  pinning the blocking mode causes. And "a thread parked in a drain costs a
  collection nothing" is still an assertion: no collection was instrumented.

- **A host must not pin a managed array across an ABI call whose completion depends
  on another thread of that host.** This is a new rule and it comes from a deadlock,
  not from a slowdown. The java binding held `GetPrimitiveArrayCritical` across the
  whole blocking call; a critical section blocks the collector, the peer was a
  grpc-java server in the same process which must allocate to answer, so a collection
  needed in that window waited on a critical section that waited on the server that
  waited on the collection. **It survived the large payload by timing and hung on the
  first small one.** Fixing it also changed the core's CPU figures at
  concurrency, so the earlier figures were contaminated as well as unsafe.

  The rule generalises past RPC and past Java: the pinned-buffer optimisation 7.1
  makes possible on decode is safe precisely because `ak_parse_*` makes **no upcall**
  and completes without any other host thread, which is what "the wire is handed
  over under a critical section and never copied" depends on. A blocking RPC call is
  the opposite case and must not be given the same treatment.
- **At least one mode in which the caller waits in the host language.** Blocking
  in a native frame from a virtual thread pins its carrier; what fixes that is
  parking in Java on a future, which the callback mode already provides. The
  requirement on the ABI is this weak and this general.

  **Checked by the java slice, and it is the one item on this list only a JVM
  slice could settle.** Eight virtual threads each wait a fixed W on a scheduler
  of known parallelism P: if the carrier is pinned the run takes `ceil(N/P) x W`.
  This is a scheduling test whose observable is wall time, not a performance
  result. At 1, 2 and 4 carriers the blocking mode's run time followed the pinned
  prediction, and the mode parked on a future stayed at about one W. So **the
  completion callback is not a convenience, it is what makes this
  ABI usable from the idiom Java is moving to**, and a host that offers only the
  blocking mode is not conformant in any useful sense on JDK 21 and later. It
  needs no RPC stack to reproduce (the question is where the waiting happens),
  which is why it was cheap and why nobody had done it. (`logs/java/pinning.log`.)

**Not offered: the host executor slot.** Built and measured on two runtimes,
it earned its complexity on neither (settled in the draft; timing evidence is
container instrumentation, re-checked in the campaign). Two structural findings from building it are
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
| a scatter/gather decode | built and timed (container instrumentation), and it would give up the property that a span is an offset into one buffer, which is what the JVM needs. Renting the flatten buffer recovers most of it in three lines |
| a host executor slot | section 7 |
| a thread-local anywhere | a hidden global with a re-entrancy hazard; the context is the replacement |
| a tape, or any positional value stream | the fastest JVM encode arm in container instrumentation, and refused on architecture: a tape is a wire format, with a grammar to specify, version and debug across two languages, which is what protobuf already is |
| batched submission, call fusion | no stable gain (settled in the draft; timing evidence is container instrumentation, re-checked in the campaign) |
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

   **Built once, and the positive control is worth more than the obligation.** The
   rust slice's suite runs two shapes in sequence and then 2, 4 and 8 threads with a
   context each, phases offset, every encode byte-compared against a
   single-threaded reference: **0 wrong of 2,840 encodes and 2,840 decodes**, so the
   codec half having no shared mutable state is now a measurement. The control that
   plants the defect does **not** report wrong bytes (it aborts the process), which
   is obligation 6.
6. **A planted panic must arrive at the boundary as `AK_ERR_PANIC`, not as an
   abort.** Section 5 makes `catch_unwind` mandatory at every entry point and
   nothing enforces it; the one slice that provoked a panic in the core found the
   process gone. The test is a deliberate panic behind each entry-point family with
   the host asserting a code came back, and it is a gate rather than a measurement:
   a codec that aborts its host on a misuse the host can commit cannot replace a
   library that throws.

## 13. Open decisions

Each blocks something. None is settled by a measurement that exists today.

1. **Is every mechanism free under the C++11 floor? REOPENED, to be decided from
   the campaign (W13).** It was recorded as answered by the C++ slice
   (`findings/cpp.md` section 2; logs cited: `bench_a17_shared.log`, `tax.log`).
   The adversarial review found that the table this rested on matches no
   committed log, and that the batching verdict flips in the cited log (FIX-PLAN
   R-C1, verified); its figures were container timings in any case. The
   mechanisms the slice described are kept, as the list the campaign prices:

   **The group's fill, which costs the HOST rather than the boundary.** The fill
   is paid per element, on M1's absent path as well, where a whole protobuf
   encode is small. That is the P1.3 inversion, which C++ reproduced as Rust did.
   Decision 9's candidate (the sparse fill) is the fix to measure beside it.

   **String as data**, which saves one reverse crossing per string. The payloads
   with no strings or almost none are its controls.

   **The batching predicate, whose sign depends on the host's crossing price.**
   The crossing was priced up with a calibrated delay in front of every forward
   entry-point call, and on P2.2 the delta between the unbatched and batched arms
   changed sign as the delay grew (section 6). **This document carries the
   crossover, not a per-host verdict**: a verdict from one host would be wrong
   for the others, and a slice that reports only its own sign has not answered
   the question. The slice's first mechanism story ("batching wins where the
   crossing count per element explodes") did not survive its own table and was
   withdrawn rather than patched.

   **A fourth mechanism nobody listed**: section 4's removal of the declared
   expansion bound forces a two-pass blob write. See the addition to section 4.

   **Blocks**: any statement of what these four mechanisms cost under the C++11
   floor. It does not block the ABI shapes: the two amendments that came out of
   it (section 4's fast path, section 6's batching sentence) and decision 13
   stand on their mechanisms.
2. **Which decode family does each binding take** (7.1), and is the single
   parameterised emitter actually buildable?

   **Half answered, and the unanswered half is now the more important one.** The
   emitter is buildable: the rust slice emits one `dec_walk` once and instantiates
   it twice, the families differing in a macro body, the entry point's prologue and
   epilogue and one argument. Its structural control is that pull writes a record
   exactly where push makes a reverse call, so the counts must be equal, and they
   are, to the digit, on all thirteen counted payloads, with pull's reverse count
   measured at **zero** everywhere. **Pull removes the upcalls; it does not reduce
   them.**

   **And the java slice has now built the pull arm on the host where it matters, so
   the practical half is answered in the draft: the ABI carries BOTH families.**
   Which family a JVM binding uses is a binding choice for the campaign to
   inform; structurally, pull makes no upcall, so the JVM buffer can be passed
   under a critical section without a copy. Reverse crossings are **zero on every payload in both
   deliveries**, where push makes 3,501 on P2.2; forward is 2 for the walk delivery
   whatever the message size, and 1 plus one per 32 KB chunk for the drain.

   **In container instrumentation the M2 decode regression the branch had carried
   since the first Java report did not appear on the pull arms** (to be measured in
   the campaign). Paired against push, pull was never slower on any payload. Against
   the no-boundary control (arm R, generated Java) pull tied rather than won on the
   M2 payloads, where push lost; P6.1 went pull's way and P1.3 and P5.1 went arm R's.
   So on that evidence **pull does not make the C ABI beat a generated Java codec on
   decode; it stops the C ABI losing to one.**

   **The drain copy was not visible on the JVM** (`ffi-pull - ffi-pull-walk`
   straddled zero on all sixteen payloads; container instrumentation). **So a host
   that finds the walk delivery awkward may drain instead**, which is a better answer
   for the specification than either delivery alone, subject to the campaign.

   **One structural consequence worth more than the ratios.** Because `ak_parse_*` makes
   no upcall *by construction*, the JVM binding can hand the wire over under
   `GetPrimitiveArrayCritical` and never copy it into native scratch, the thing the push
   family forces, since an upcall and a critical section are mutually exclusive. The shim
   pushes no callback frame at all, so a future callback cannot be added without someone
   noticing the rule was broken. That is the design constraint 7.1 was written around,
   now built rather than argued.

   **What is still open**: C++, C# and Python have push arms only. The two hosts where
   that matters most are C# (where the two crossing directions may be close in price)
   and Python (where the ABI's crossings
   are already 0.01 per element and the answer may be that neither family is the
   question).

   **What the rust slice CAN hand a managed host is a re-pricing kit, and it built
   one.** The crossing arithmetic is a property of the descriptor: push is per
   ELEMENT and pull is per MESSAGE, so P2.2 goes from **3,501 reverse calls to 16
   forward**, or to **3** if the host sizes one drain chunk to `ak_bdr_footprint`.
   Three is the floor for every payload in the set, and the chunk size is the only
   knob the host has: it trades crossings against how much of the response is
   materialised at once, which is the bound 7.1 gives the host in the first place.
   The two costs that replace the upcalls are decomposed so another host can price
   them without building the arm: **materialisation (`ak_parse_*` alone) and the drain
   copy**, each a share of a push decode that depends on shape (the Rust split is
   container instrumentation). A host therefore trades 7.004 upcalls per element
   against 3 to 16 forward calls per message plus those two terms.

   **On a host whose reverse call is cheap, pull was expected to lose and did not
   clearly lose** (Rust; container instrumentation, to be measured in the campaign).
   The mechanism offered: a push reverse call goes through a vtable slot reached
   across the shared object while the replay's equivalent is a local call over a
   buffer already in L2. An opaque-replay arm clears the obvious objection that the
   replay was being inlined.

   **Where pull loses, a byte table predicts it.** The rows where pull was slower in
   container instrumentation are P1.3 and P6.1, and on P1.3 the record stream is
   **63.6 times the wire** (38,488 B to describe a 605 B message), because a record
   carries an absent element's whole fixed group. **Pull's cost should track the ratio
   of record bytes to wire bytes, which is a property of the SHAPE**, and every payload
   whose ratio is below 1 was at or under push. That is the rule a binding author can
   apply to a shape before measuring it, and it makes the absent path the
   one place where pull and decision 9 have to be reasoned about together.
3. **Where does UTF-8 get checked? SETTLED: not on encode, and rejected on
   decode.** Asked three times. The first two framings ("fail or substitute",
   then "validate or trust the host") both assumed the check belongs on the encode
   path, and the third showed it does not.

   **Encode does not check.** A `string` field is a length prefix and a byte copy,
   so validity changes nothing about the framing and the check buys the encoder
   nothing. The decoder cannot trust the bytes whatever the encoder did, since they
   arrive off a wire anything may have written, and proto3 puts the obligation on
   parsers for that reason. `ak_tc_utf8` is therefore `ak_tc_bytes`: one memcpy,
   no validation, no trust extended to anybody and so no contract a host can be
   wrong about.

   **Decode rejects, and rejecting should not be a cost.** On the string path alone,
   validate-and-reject was no dearer than a lossy decode, and with a SIMD validator
   it was cheaper on every content set (container instrumentation; to be measured
   in the campaign). The reason is structural: a lossy conversion **already
   validates** (it scans to decide what to replace), and its recovery path does
   more work than failing.

   **And it makes the comparison fair.** prost rejects malformed UTF-8, so a
   rejecting decode makes both sides do the same work. The old arrangement paid
   for a validator to get a weaker guarantee, on both sides of the boundary at
   once.

   **What survives.** The malformed-input policy stays for the *converting*
   transcoders (`ak_tc_utf16`, `ak_tc_latin1`, `ak_tc_ucs4`): an unpaired surrogate
   is not representable in UTF-8, that is a conversion question rather than a
   validation one, and section 12.2's transcode pair is still a pair. **On CPython
   two of those three compete with the interpreter's own cache rather than with
   nothing**: reading a `str`'s UTF-8 is cheap for ASCII and much dearer for
   Latin-1 and above-U+00FF when the object has no cached UTF-8 yet (container
   instrumentation), which is exactly the state of a string that came off the wire, and CPython keeps the
   result afterwards. So `ak_tc_latin1` and `ak_tc_ucs4` are worth what they save
   against a *first* read, not against a steady-state one, and on ArmoniK's actual
   content (ASCII GUIDs) the passthrough is the common path anyway. Encode
   validation survives only as an **opt-in diagnostic mode**, because it surfaces a
   bad string at the caller that produced it rather than at a receiver in another
   language where a conformant parser rejects the whole message; that is roughly
   what protobuf C++ does today, and it is not paid on every encode by default.

   **The core's validator was cheaper than the one the host is already running.**
   Compared against protobuf C++'s own `IsStructurallyValidUTF8` (the validator the
   incumbent runs on every `string` field it parses, already linked into every arm,
   so no configuration claim has to be believed), the core's table validator came
   out cheaper on ASCII, Latin-1 and wide content (`logs/cpp/utf8.log`; container
   instrumentation, to be measured in the campaign). If that holds, decision 3's
   decode-side check is not a cost the core imposes on a host that did not have
   one.

   **An earlier figure is withdrawn, and it erred in the flattering direction.** Its string set included `ResultRaw.opaque_id`, which is a `bytes`
   field: proto3 puts no UTF-8 requirement on it, the codec reaches it through
   `ak_tc_bytes`, and no validator ever sees it. In the ASCII set its values are
   arbitrary bytes, so the check arm rejected on the first bad byte and did *less*
   work than a validation, so the published ASCII row understated the cost of
   validating. Found by a differential test asserting that everything it validates
   is valid, which the timing table had never done.

   **Two notes on how that validator was arrived at, because they generalise.**
   Byte identity cannot see a validator defect at all: a manifest is made of
   things that *encode*, so it carries no malformed input, and a validator that
   accepts an unpaired surrogate passes every gate the branch has; it took 17.78
   million differential checks against an oracle written from RFC 3629, one range
   per line, which is not one of the implementations under test. And a textbook
   DFA turned out **slower than the scalar form on wide content** (container
   instrumentation), because its state is a serial dependency and the branches it removes were being
   predicted correctly anyway. What wins keeps the scalar shape and drops the
   code-point arithmetic: validation needs ranges rather than values, and every
   range constraint in UTF-8 is a function of the lead byte alone.

   **Where a fast validator earns its place is decode**, not encode, which is where
   the `simdutf8` dependency and its runtime-dispatch floor question go with it.

4. **Is `ak_span.coder` in the shared struct or out?** It is a JVM-specific hint
   in a struct every language reads.
5. **What the grow path actually costs now that nothing is reserved from a
   declared bound** (section 4). The codec hands the transcoder whatever the
   buffer has left, so a grow should be rare, but no slice has measured the rate
   or what a grow costs when the length prefix has to be resized after it. The
   first slice to build the encode path answers it, and P2.4 is the payload for
   it. **This is the one decision created by v1 rather than inherited.**

   **Answered. Keep the learned width** (settled in the draft; timing evidence is
   container instrumentation, re-checked in the campaign). The Rust slice counted
   it per site, which an aggregate cannot do. On every uniform payload (P1.2, P2.2, P2.3,
   P2.5) a warm context misses **zero** times and moves **zero** bytes. On P2.4,
   built so that a per-site width is wrong on every element, it misses once per
   element and memmoves 980,938 bytes of a 981,222-byte output: the whole payload,
   once, every encode. Isolated against two size-matched uniform arms rather than
   attributed, and against prost as a floor for the construction's own
   non-linearity, **the mechanism's cost appeared small on the payload built to
   defeat it** and there are no moves elsewhere (container instrumentation; to be
   measured in the campaign). Each move is a sequential in-cache memmove of a
   ~12 KB element body. **Zero grow-callback invocations on any payload**, which is what handing the transcoder the whole
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
9. **Does the by-value group need an empty-element path? ANSWERED in Rust: yes,
   and it is a host-side fill change rather than an ABI change** (settled in the
   draft; timing evidence is container instrumentation, re-checked in the
   campaign). The group carries the whole singular subtree unconditionally
   (section 6), so on P1.3, where every element encodes to nothing, the fixed cost
   had nothing to amortise against and `core-ffi-rust` fell behind prost in both
   directions where the no-boundary control stayed ahead.

   **The cost is the fill, not the boundary, and that was audited rather than
   inferred.** The per-element figure was obtained by subtracting the no-boundary
   arm, which invited the objection that the subtraction charges an inlining
   advantage to the interface. With two further arms (`#[inline(never)]`, and a
   `black_box`ed function pointer that also defeats devirtualisation and constant
   propagation), the inlining term was negligible against the group term on both
   encode and decode. With LTO off the traversal was never inlined into the caller
   in the first place, so both arms already paid an indirect call.

   **The candidate: the host memsets the element-group chunk once and assigns only
   the fields that differ from the default.** On P1.3 encode the inversion
   disappeared; P2.2, the shape the control plane actually moves and the row set up
   to decide against it, showed a small saving; P1.2 and P2.5 were neutral within
   spread.

   **What it does and does not touch.** It does *not* reverse section 6's
   no-reset-between-elements property: the array is the host's own chunk buffer and
   the codec still resets nothing. Only the host's fill strategy changes, from an
   unconditional store per field to a bulk clear plus a conditional store. So the
   total-fill invariant as the *codec* relies on it is intact, and what would change
   in this document is the sentence telling the host how to fill.

   **Why it is not adopted here.** A bulk clear of a struct array and a conditional
   store cost something quite different in a managed host, and this is exactly the
   kind of mechanism whose value differs by runtime by construction, as the batched
   element run's does. **Blocks: nothing. ANSWERED in the draft: three hosts
   agree.** Rust and C++ settled it first; **the java slice extended it to a
   managed host**, which is what this decision was waiting on: on the absent path
   the sparse fill came out ahead of the total fill with a clean sign, the
   absent-path inversion against protobuf-java disappeared, and every other
   payload stayed inside the drift bar (container instrumentation; to be measured
   in the campaign). **The sparse fill becomes the specified path**, with the
   wording corrected below; the total fill stays legal for a host that prefers it.

   **The candidate's own wording is wrong, and the C++ slice found it.** "Clear the
   chunk" is what `rust_abi.py` emits, and it clears the whole 32 KB arena
   regardless of how many elements will be filled: O(arena) where the fill is
   O(elements). That made the candidate look like a large loss on payloads with few
   elements per chunk (P1.1, P2.1) while it still won on P1.3, so the candidate
   looked refuted and was not. Clearing only `min(n, chunk)` elements removes the
   inversion and keeps the wins (container instrumentation). **If
   decision 9 is adopted, the sentence is "clear the elements you will fill".**
   The rust slice could not see this: it ran P1.2, P1.3, P2.2 and P2.5, and the
   effect needs a payload with few elements per chunk.

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
   7's note says is the one still available on decode: one construction per
   element is allocation, and allocation appeared to dominate decode (container
   instrumentation).

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

   **Status: the owner has not decided** (FIX-PLAN D4). Both behaviours, drop and
   retain, are to be built in the core and in the generator, as a generator option,
   and both are measured in the campaign (FIX-PLAN WP3 item 21, WP5). Dropping
   changes behaviour for four of the five languages (C#, Java, C++, Python), whose
   incumbents retain unknown fields; only Rust's incumbent already drops them. The
   product question that bears on it is whether ArmoniK re-encodes anything it
   decoded (FIX-PLAN R-A6). It was found by a shape-coverage vector rather than by
   a benchmark.

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

13. **Should the facade be able to BORROW a decoded string rather than own it?**
   Opened by the C++ slice, on a decode effect seen in container instrumentation.
   Nothing in this document changes to allow it: `ak_span` is already an
   offset into the buffer the host handed in (section 4), and section 7 already
   tells the host to resolve spans against a base pointer it holds. What is open
   is what the **facade** promises.

   **What it may be worth.** A borrowed-string facade arm in C++ (same codec, same
   ABI, same generated call text, only the destination type changed) moved decode
   from behind upb to level with it on the element-bearing payloads. P6.1, one
   string and five packed scalar arrays, barely moved, which is the control that
   says the arm measures the copy and not something else. (Container
   instrumentation; to be measured in the campaign.)

   **It is not a C++ mechanism.** The provenance table below already records that
   decode spans as offsets into the host's buffer are what a 4 MB download on the
   JVM relies on. Three hosts, one mechanism, and the branch held both halves
   without connecting them.

   **The java slice then tried the same mechanism on the JVM, and a managed host
   got less of it.** `ffi-borrow` moved P1.2 and P1.1 in the same direction as C++,
   by less; on the container-heavy P2.2 it straddled zero, which is the branch's own
   container-construction bound seen from a third host (container instrumentation).
   So the contract has to be drafted against a host that gets less than C++ does,
   not against the best case.

   **And the Python premise this decision was carrying is wrong.** The open question
   supposed upb may already borrow at the Python level, which would make the option
   moot there. It does not: upb aliases into its input buffer in C, but a Python
   `str` is a fresh object built on **every** attribute read and nothing is cached,
   so a second full read of the *same* upb message pays nearly all of the
   materialisation again (container instrumentation). **A caller that reads its
   response twice pays upb's materialisation twice and the facade's once.** The borrowed span is therefore open
   in Python and the incumbent has not taken it; what a borrowed Python string *is*
   has no draft, and that is the blocker rather than the measurement.

   **What is actually open is the lifetime contract.** A borrowed view is valid
   only while the input buffer lives, which this document has never written down;
   in C# and Java it interacts with pinning, in C++ it means a facade type that is
   not `std::string`, and a hybrid facade that borrows some fields and owns others
   has a public surface nobody has drafted. **Settled by**: drafting that contract
   and pricing what a host pays to honour it in the campaign. **Blocks: nothing
   today; it is an additive option. Which decode claim the campaign can support
   depends on it: the report states whether a decode figure was taken with
   borrowed or owned spans.**

**Settled since the first draft**, kept here so a reader of an earlier version
does not look for them: the accessor error channel is now section 5 rather than a
gap, `max_bytes_per_unit` is gone rather than specified, and configuration
precedence is stated in section 3 rather than carried as a decision.

## 14. Provenance

Every amendment, what motivated it, and what it turns on. A slice that wants to
revisit one starts here rather than re-deriving it. Where the motivation was a
timing, it was taken in a container and is instrumentation, to be measured in the
campaign; the figures themselves were removed (FIX-PLAN WP2).

| Amendment | From | What it turns on |
|---|---|---|
| by-value group carrying the singular subtree | C#, confirmed on JVM | decode on P1.2, and a JVM control without it (instrumentation) |
| string as data in the group (the triple) | C# and Java | the largest single change on JVM and .NET encode (instrumentation); one reverse crossing fewer per string |
| lengths in source code units | Java | the ratio-over-bytes form cost a hardware divide per string |
| transcoder growth callback, and no declared expansion bound at all | Java, then v1 | the callback closed silent wire corruption that the retry loop it replaced allowed; v1 drops the declared bound with it (decision 5) |
| packed scalar as the host's own array | C# | 2,001 crossings against 25,201 |
| one element entry point with a count | C# | less compiled code, one protocol instead of two |
| host-driven batched element runs, leaf form default | Java | forward crossings saved; worth differs by runtime (instrumentation) |
| batching predicate, transitive, from the descriptor | C# | 7 crossings per task against 43 |
| bounded arena in the context, flushed on a foreign tag | C# | correctness on legal interleaved wire, plus re-entrancy |
| decode spans as offsets into the host's buffer | C# and Java | an offset stays meaningful after a JNI critical section is released, which the 4 MB download path relies on |
| pull-family drain | Java | zero upcalls, so the wire can stay under a critical section; decode against protobuf-java (instrumentation) |
| learned length-placeholder width, per context | Java | a global table is a data race; aggregate throughput against it (instrumentation) |
| direct arguments for bulk bytes | Java | no staging copy on a 4 MB upload on the JVM (instrumentation) |
| plain exports rather than a table | C# | interface size and failure mode |
| no map case | C# | a per-entry cost, paid deliberately (instrumentation) |
| group layout export and assert, one ABI version | C# and Java | insurance against the worst failure mode a by-value ABI adds |
| RPC: handle on the blocking call, metadata, deadline, status code | Java | a retry policy is a function of the status code |
| explicit `ak_init`, one-shot installs owned there | base design, v1 | the crypto provider, the log and tracing bridges and the panic hook cannot be installed late, and two of the three are one-shot per process |
| error channel in the context, guard generated | C#, then v1 | an unguarded accessor terminates the process; the guard has a per-accessor cost, and every published margin was measured without it |
| completion queue with one drainer; no executor slot | Java, C# | the queue trades one reverse call for one forward call and lets a virtual thread wait in Java; the executor slot earned nothing on either runtime (instrumentation) |
