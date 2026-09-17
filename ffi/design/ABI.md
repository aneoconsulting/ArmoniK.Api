# The reconciled C ABI

**Status: draft for agreement (W1).** This is the specification every slice is
built against. Until it is agreed, no slice starts; once it is agreed, a slice
that disagrees with it raises a finding rather than diverging quietly.

It merges three sources. The [base design](https://claude.ai/code/artifact/d29ed568-05eb-4ded-b22a-b1db669a56fb?sk=k-raOgdhnjAvYmpd0YdSGA)
supplies the layering, the RPC half, lifetime and the streaming contract. The
[C# finding](https://claude.ai/artifact/WYD94FSYuq1Nxjdu6WHbtS?sk=aeQYJo8cccAcZsFgdTXRkQ)
section 4 replaces the codec half, which it measured as a regression in the base
design's form. The [Java finding](https://claude.ai/artifact/YFSNVzYD41C1TsHmANLcBu?sk=MJlBPbq3WV9R4dxdhAv6Og)
section 4 amends that again and, on decode, contradicts it; section 5 below is
where the contradiction is resolved rather than averaged.

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
on encode. On decode they do not, and section 5 settles it with two delivery
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

## 3. Common vocabulary

```c
/* Every entry point reports out of band, including the void ones: a Rust panic
   crossing extern "C" aborts, so catch_unwind is mandatory and a caught panic
   needs somewhere to put its message. There is no thread-local last_error. */
typedef struct { int32_t code; const char *msg; uint32_t msg_len; } ak_err;

/* Encode: a string or bytes field as DATA inside the group, never a call.
   `len` counts SOURCE code units, never bytes and never characters.
   `data` must stay valid for the duration of the codec call.
   `tc == NULL` means the field is absent. */
struct ak_str { const void *data; size_t len; const struct ak_transcoder *tc; };

/* Decode: an OFFSET into the buffer the host handed in, plus a byte length.
   8 bytes where a pointer pair was 24, and still meaningful after a JNI host
   has released a critical section, which is what lets that host build a String.
   `coder` is an optional host hint (see 5.4). */
struct ak_span { uint32_t off, len; uint32_t coder; };

struct ak_transcoder {
  uint32_t max_bytes_per_unit;   /* utf8 1, latin1 2, utf16 3, ucs4 1, bytes 1 */
  uint32_t reserved;
  int32_t (*transcode)(const void *src, size_t len, uint8_t *dst, int32_t cap,
                       ak_grow_fn grow, void *grow_ctx);
};
const struct ak_transcoder *ak_tc_utf8(void);    /* Rust String, Go string */
const struct ak_transcoder *ak_tc_utf16(void);   /* .NET string, JVM String */
const struct ak_transcoder *ak_tc_latin1(void);  /* JVM compact, CPython 1-byte */
const struct ak_transcoder *ak_tc_ucs4(void);    /* CPython 4-byte */
const struct ak_transcoder *ak_tc_bytes(void);   /* memcpy, no validation */

uint32_t ak_abi_version(void);          /* checked once at load */
size_t   ak_sizeof_group(uint32_t id);  /* asserted against the host's own sizeof */
```

**Why the transcoder is data rather than a callback.** There are not many
representations a host actually holds, so the core implements all of them once
for every language, and every managed host stops maintaining a UTF-8 encoder.
`max_bytes_per_unit` is a field and not a call because the codec needs it once
per string to size its reservation, and a call there puts back the crossing this
exists to remove. The unit form, rather than a ratio over a byte length, is worth
3.0 to 14.1 percent of an encode: the two are the same function (checked at 4,097
lengths, zero disagreements) but the ratio form costs a hardware divide by a
runtime value, once per string, on a schema with 174 string fields.

**The grow callback is what makes the bound a hint.** A host may supply its own
transcoder and may be wrong about its own bound. With a plain bound as a
correctness contract, an under-declaring transcoder produces *silent wire
corruption*: measured, a transcoder that doubles every byte and declares 1
returned success and wrote a zero-length prefix followed by zeros. With the
growth callback the transcoder asks for room like anything else, and the bound
drops to a hint for the first reservation. It also measured slightly faster than
the codec-side retry loop it replaces (median 0.957). One check is not removable:
the codec refuses a returned count larger than the capacity it gave, because
nothing can make a transcoder that writes past its buffer safe.

## 4. Encode

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
                    const struct ak_transcoder *tc);       /* declines, never allocates */
void    ak_blob_reserve(void *ctx, int32_t want);          /* may allocate */
void    ak_run_i64(void *ctx, const int64_t *p, size_t n); /* one per host layout */
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
context, so `bool` and `enum` need no cases. A host with no contiguous layout
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

**The fill must be total.** Every scalar, every count and all three words of
every `ak_str` are assigned unconditionally, and the presence word is assigned
rather than OR-ed. In exchange the codec does not reset the element group between
elements, worth 5.4 ns per `ResultRaw` and 24.4 per `TaskDetailed`. This is an
invariant and it belongs in the header: a partial fill does not fail, it silently
inherits the previous element's value.

## 5. Decode

**Decode needs less machinery than encode, not more**, because the codec already
owns the bytes: the span points into the buffer the host handed in, so there is
nothing to reserve, size or transcode.

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

### 5.1 Two delivery families, one traversal emitter

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
int32_t ak_decode_ListResultsResponse(void *obj, const uint8_t *buf, size_t len,
                                      const struct ak_dvt_ListResultsResponse *vt);
struct ak_dvt_ListResultsResponse {
  void (*apply)      (void *obj, const struct ak_dfix_ListResultsResponse *fx);
  void (*add_results)(void *obj, const struct ak_dfix_ResultRaw *elems, int32_t n);
};

/* pull: the context is host-owned, so the host can hold two decoded responses,
   read what it is paying, bound it and release it. */
ak_bdr_ctx *ak_bdr_ctx_new(void);
int32_t     ak_bdr_reserve(ak_bdr_ctx*, size_t bytes);
size_t      ak_bdr_footprint(const ak_bdr_ctx*);
void        ak_bdr_ctx_free(ak_bdr_ctx*);
int32_t     ak_parse_ListResultsResponse(ak_bdr_ctx*, const uint8_t *buf, size_t len);
int32_t     ak_bdr_drain(ak_bdr_ctx*, uint8_t *dst, int32_t cap, size_t *cursor);
```

### 5.2 Batching predicate, and why batches never nest

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

### 5.3 The arena, and the flush that makes it correct

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

### 5.4 Two rules for a facade author

**Resolve spans against the base pointer you already hold.** The host pinned the
buffer to make the call, so an offset is one add and then the same fused
transcode. Indexing the managed array instead measures 14 percent worse, and it
is the obvious thing to write.

**A batched add may be called more than once per field.** Append; never size to
the count you were handed.

`ak_span.coder` is a host hint (whether the bytes are Latin-1, so a JVM host can
take a straight compact copy). It is a host-specific field in a shared struct and
is **optional in this specification**: see open decision 4.

## 6. Bulk bytes: the direct-argument path

A small range of `ak_str.data` values is reserved as sentinels meaning *this
field is a direct argument of the call* rather than a pointer into staging. It is
one sentence in the specification and the one unambiguous win on the JVM: a
multi-megabyte result upload at 0.16 to 0.34 of protobuf-java, against roughly
parity for a staged path. `critical(true)` on FFM, `GetPrimitiveArrayCritical` on
JNI; C++ and C# never use it and pay nothing for it.

It generalises untested: it is built for one field of one root message, nothing
tests it on a nested message, and **a direct field declared on a message that
does make a reverse call should be a generator-time refusal** and currently is
not.

## 7. The RPC half

Unchanged in shape from the base design, and it is the half whose case is
behavioural rather than performance: one retry set, one backoff, one TLS
configuration, one cancellation contract, enforced rather than copied.

```c
ak_runtime *ak_runtime_new(ak_err*);
ak_context *ak_context_new(ak_runtime*, ak_bytes_in config_json, ak_log_fn, void*, ak_err*);
ak_client  *ak_client_new(ak_context*, ak_err*);

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

**`worker_threads` comes from config with a small explicit default.** Never
`Runtime::new()`: Rust reads the cgroup quota, so `cpu: 500m` rounds down to one
worker while a requests-only pod takes every CPU on the node (measured: 23
threads, 1.49 GB of virtual address space, idle). Two workers to four on two
vCPUs cost 2 percent of throughput and doubled to quadrupled the p999, so a
client that silently takes a worker per CPU does not look slow, it looks erratic.

**The streaming concurrency contract** is unchanged from the base design:
`send || recv` on one call allowed from any threads, `send || send` and
`recv || recv` refused with `AK_INVALID_STATE` through a per-direction atomic and
a try-lock, `close || anything` allowed and must unblock, `destroy || anything`
forbidden. Under callback delivery, "returned" means the completion has fired.

## 8. Lifetime, versioning and load-time checks

- **Every handle is a plain pointer**, created and destroyed explicitly by the
  host, which is how every C library works and what RAII, `SafeHandle` and an FFM
  `Arena` all expect.
- **`close` and `destroy` are separate operations** and collapsing them is not
  merely risky but unimplementable: a woken `recv` re-acquires the mutex inside
  the object to finish waking, which TSan catches as a use-after-free with no
  ordering of stores that avoids it.
- **One `ak_abi_version()`, checked once at load.** The whole ABI versions as a
  unit; not a size field per table. The host-called direction needs no version,
  because a mismatch there is a link failure.
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

## 9. Deliberately not in the ABI

| Not in it | Why |
|---|---|
| a map case | a map is a repeated field of a pair message, and the repeated-message path handles it both ways. Specialising it into two strings is correct for one instantiation of one container |
| a scatter/gather decode | built and measured at 0.90 to 0.95, and it would give up the property that a span is an offset into one buffer, which is what the JVM needs. Renting the flatten buffer recovers most of it in three lines |
| a host executor slot | section 7 |
| a thread-local anywhere | a hidden global with a re-entrancy hazard; the context is the replacement |
| a tape, or any positional value stream | fastest measured encode arm on the JVM (0.99 to 1.16) and refused on architecture: a tape is a wire format, with a grammar to specify, version and debug across two languages, which is what protobuf already is |
| batched submission, call fusion | under one percent at best and not stable in sign |
| a runtime schema fingerprint | both sides ship from one release, so a disagreement is a codegen bug: it belongs in CI as a build-time subset check, not in a runtime guard |

## 10. Conformance obligations this ABI creates

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
5. **A concurrency suite that runs at least two payload shapes** of at least one
   message type, with threads run in sequence as well as together, and every
   encode asserted against a reference rather than counted. A suite with one
   shape reports zero wrong bytes with a per-thread-state defect present and
   absent alike; two shapes find it in twenty encodes out of twenty.

## 11. Open decisions

Each blocks something. None is settled by a measurement that exists today.

1. **Is every amendment free under the C++11 floor?** The amendments were
   motivated by managed hosts. That C++ pays nothing for the group, the triple
   and the batching predicate is currently an argument. Settled by the C++ slice,
   or earlier if it is cheap to check. **Blocks: freezing this document.**
2. **Which decode family does each binding take** (5.1), and is the single
   parameterised emitter actually buildable? Settled by the first two slices that
   pick different families.
3. **UTF-8 passthrough: validate-and-fail, or validate-and-substitute?** Fail
   keeps the bound at 1 and gives C++ a memcpy with an exact reservation;
   substitute makes it 3 and has the cheapest-boundary language make the largest
   reservation. The Java slice chose fail.
4. **Is `ak_span.coder` in the shared struct or out?** It is a JVM-specific hint
   in a struct every language reads.
5. **The worker path.** Either Rust hands the facade the raw `ProcessRequest`
   bytes and the facade decodes them itself, which is two decoders over one
   buffer that can silently disagree, or Rust decodes once and exposes typed
   getters, which puts the schema back into the boundary and contradicts the
   fixed-size property everywhere else. Obligation 10.4 covers the first.
6. **The accessor error channel.** Repeated fields have one (every loop returns
   `int32_t` and the codec rolls back to recorded positions). The group's fill
   does not, and the natural place is a status word in the group, which already
   crosses once per message. Every published margin is measured *without* the
   generated try/catch that a managed host requires, so adopting this costs a few
   percent of encode on string-heavy messages and the slices must re-measure.
7. **Decode recursion limit.** Rust holds the reader and recurses in Rust, so
   prost's `RECURSION_LIMIT` does not apply and the built codec has none. The
   in-scope schema is acyclic with static depth 6 and unknown nested fields are
   skipped without recursing, so it is not reachable today and nothing enforces
   that.
8. **Message size limits.** `WorkerServer.h` sets `SetMaxReceiveMessageSize(-1)`
   today; tonic defaults to 4 MiB. Without an explicit per-direction knob,
   migration day gives `RESOURCE_EXHAUSTED` to every customer whose payloads
   exceed 4 MiB, which for an HPC orchestrator is normal.
9. **The diagnostic contract.** Five distinct transport failures currently render
   as one string. `ak_err` must carry a machine-readable failure class and the
   flattened source chain, and someone must own `tracing::set_global_default`,
   which is one-shot per process and therefore cannot be retrofitted.
10. **Configuration precedence**, which is already decided by
    `Configuration.cpp` and must be reproduced: an explicit `set()` beats the
    environment, the environment beats JSON and defaults. Inverting it turns a
    pod-spec environment variable into a silent certificate-verification bypass.

## 12. Provenance

Every amendment, what motivated it, and where the figure lives. A slice that
wants to revisit one starts here rather than re-deriving it.

| Amendment | From | The figure it turns on |
|---|---|---|
| by-value group carrying the singular subtree | C#, confirmed on JVM | decode 1.192 to 0.955 on P1.2; the JVM control without it is 1.22 to 1.64 times worse |
| string as data in the group (the triple) | C# and Java | the largest single change: 1.12 to 1.64 times on JVM encode, 25 to 36 percent on .NET |
| `max_bytes_per_unit` per code unit, not a ratio | Java | 3.0 to 14.1 percent of an encode |
| transcoder growth callback | Java | closes silent wire corruption; 0.957 against the retry loop it replaces |
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
| completion queue with one drainer; no executor slot | Java, C# | the queue is the best arm on virtual threads; the executor slot earns nothing on either runtime |
