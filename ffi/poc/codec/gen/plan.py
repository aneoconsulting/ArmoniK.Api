"""The rule layer: the IR lowered to language-neutral PLANS. FIX-PLAN WP5 item 2.

Owner position 5 (`design/FIX-PLAN.md` section 0): there is ONE generator, and the wire
rules are written ONCE. This module is where they are written. Every backend -- the Rust
core behind the C ABI, the Rust core-native control, and (next) the C++, Java, C# and
Python backends -- renders what this module decides and decides nothing about the wire.

====================================================================================
THE CONTRACT: what a plan contains, and what a backend may and may not decide
====================================================================================

Entry point
-----------
    plan = lower(ir, Options(...))     # ir from ir.load(...) / ir.load_corpus(...)
    plan = as_plan(x)                  # x a Plan (returned as is) or an Ir (lowered with
                                       # default Options): what a backend calls

A backend imports THIS MODULE and nothing from the IR or the schema (`generate.py
--check` enforces it). Everything it needs is below.

Plan
----
    plan.options        Options(unknown, utf8, recursion_limit): the generator options
    plan.roots          the root messages, in the order entry points are emitted
    plan.messages       {name: MessagePlan} for every message reachable from a root, plus
                        one SYNTHETIC pair message per map field (ABI v1 section 11: a map
                        is a repeated field of a pair message and has no case of its own)
    plan.order          real messages in description order
    plan.abi_order      synthetic pair messages first, then `order`
    plan.enums, plan.enum_order     the enums the messages use (name -> description)
    plan.msg(name)      MessagePlan
    plan.rpc            the RPC half's fixed ABI (structs and prototypes, R-G5)
    plan.lifecycle      `ak_init` and what every binding must do with it (R-G7)
    plan.payloads       the description's payload definitions, as data (for builders)
    plan.source         which description the plan was lowered from

MessagePlan
-----------
    .name .synthetic .leaf .raw
    .fields             every FieldPlan, in TAG ORDER
    .plain              fields that are not oneof members, tag order
    .oneofs             {oneof name: [member FieldPlan, ...]}, tag order within
    .encode             the ENCODE PLAN: a list of EncStep, in TAG ORDER (R-E8). The order
                        of the list IS the order of the writes on the wire. A oneof is one
                        `oneof_member` step PER MEMBER at that member's tag position, each
                        guarded by "the case is this member", so a oneof interleaved with
                        plain fields still writes in tag order.
    .oneof_checks       [(oneof name, [member tags])]: before any write, a case that is
                        neither 0 nor a member tag is REFUSED (ERR_ABI); a host generated
                        against a newer descriptor must be loud, not silently lose a field
    .decode             the DECODE PLAN: {(field number, wire type): DecAction}. A pair
                        not in the table is an unknown field (see DECODE RULES) -- which is
                        how a known field number at the wrong wire type is handled, for
                        every kind identically (R-E2)
    .recursive          True if the message can contain itself (Nest)

FieldPlan (the descriptor, plus the rules resolved for it)
---------
    .name .tag .kind .card .of .entry .explicit .oneof .direct .value_rule .adapter_site
    .wire       the wire type the field is WRITTEN with (LEN for packed and map)
    .value      the scalar codec, one of VALUE_CODECS (None for string/bytes/message/map):
                  varint_i32   write sign-extended to 64 bits (a negative is 10 bytes);
                               read the low 32 bits as a two's-complement i32 (R-E5)
                  varint_i64   write/read the 64 bits as is
                  varint_bool  write 1 for true; read any nonzero as true
                  fixed64_f64  8 bytes little-endian IEEE-754
                  fixed32_u32  4 bytes little-endian
    .presence   how an encoder decides the field is written (see ENCODE RULES):
                  "explicit" | "implicit" | "message" | "oneof" | "direct" | "repeated"
    .utf8       True for a `string` (validated on decode under Options.utf8="reject")
    .recursive  True if this message-typed field can reach its owner again

EncStep (encode plan element)                  op, field
-------
    scalar            a singular varint/fixed field
    blob              a singular string/bytes field (presence: implicit, explicit, direct)
    child             a singular message field: LEN, then the child's own encode plan
    packed            a repeated scalar: ONE LEN run of values; empty -> nothing written
    repeated_blob     one LEN field per element, empty elements written
    repeated_message  one LEN field per element, empty elements written
    map               one LEN pair message per entry (the entry's own plan)
    oneof_member      the member, written whatever its value, iff the case selects it
    unknown_tail      retain mode only: the captured unknown runs, verbatim, AFTER every
                      known field (upb's position; protobuf-java writes them sorted)

DecAction (decode plan element)                op, field, read
---------
    set_scalar        last one wins; an explicit field also sets its presence
    set_blob          last one wins; `utf8` -> validated
    merge_child       singular message: a repeated occurrence MERGES into the one already
                      decoded (protobuf semantics), it does not replace it (R-E4)
    append_message    repeated message: each occurrence is a NEW element
    append_blob       repeated string/bytes: each occurrence a new element; `utf8`
    packed_run        the LEN form of a packed field: values until the run ends
    packed_one        the unpacked form of the same field, at the kind's own wire type
                      only (a packed int64 arriving as fixed64 is an unknown field, R-E2)
    map_entry         a pair message; a duplicate key replaces the earlier value (the
                      host's map insert); the pair's own decode plan reads key and value
    oneof_set         sets the case to this member; a scalar/blob member replaces; a
                      MESSAGE member merges if the case already is this member and starts
                      from empty otherwise (R-E4)

ENCODE RULES (stated once, applied by every backend)
------------
    * writes in tag order (the list order);
    * implicit presence: varint kinds when != 0 (bool when true); fixed32 when != 0;
      double when its BIT PATTERN != 0, so -0.0 is written and +0.0 is not (R-E3);
      string/bytes when non-empty; a message when present;
    * explicit presence (`optional`): written iff set, zero or empty or not;
    * a DIRECT-argument bytes field (ABI v1 section 8, presence "direct") is an
      implicit-presence bytes field on the wire: written iff its length is not 0 (the core
      tests `len != 0` on the direct argument; WP5 step 6, R-G13);
    * a oneof member: written iff the case selects it, whatever its value;
    * repeated scalars are packed; an empty repeated field writes nothing;
    * a map entry is an ordinary message of two implicit-presence fields, key 1 and
      value 2: an empty key or value is omitted (`ffi/schema/README.md`'s canonical form,
      the form the payload-set manifest hashes). The corpus also accepts "key and value
      always written" (protobuf C++, upb, protobuf-java); it accepts NEITHER form's hybrid
      "key always, empty value omitted" (`E-map-entry-empty`), which is why FIX-PLAN WP5's
      "key always written" is not what this layer states -- see the note in `_lower_map`;
    * map ENTRIES are written in ascending order of the key's UTF-8 bytes (code point
      order), one entry per key: the manifest's canonical form, what a Rust `BTreeMap`, a
      C++ `std::map<std::string>` and a sorted Python dict give. Where the host hands the
      entries (a C ABI binding filling a run), the binding hands them in this order;
    * lengths are minimal varints (a learned-width backend moves the body on a miss; it
      never pads, ABI v1 section 6);
    * retain mode: every message's captured unknown runs after its known fields.

DECODE RULES (stated once, applied by every backend)
------------
    * dispatch on (field number, wire type) through the table; not in the table -> the
      field is UNKNOWN: skipped (drop) or captured verbatim (retain);
    * field number 0 -> ERR_MALFORMED, on every message (R-E4, R-E5);
    * wire types 6 and 7, and an END_GROUP (4) with no group open -> ERR_MALFORMED;
    * wire type 3 opens a GROUP, skipped whole to the END_GROUP whose field number matches
      (a mismatched end is ERR_MALFORMED, an unterminated one ERR_TRUNCATED);
    * LENGTH ARITHMETIC IS 64-BIT AND CHECKED AGAINST THE REMAINING BYTES, in every backend
      (R-D1, R-G8): a length prefix is read as the full unsigned 64-bit varint -- never
      narrowed to a 32-bit or signed int before the check -- and accepted only if
      `n <= len - pos` (with `pos <= len` an invariant), never tested as `pos + n > len`,
      which wraps. A host runtime that narrows (C# to `int`, Java arm R's `pos + n > limit`
      in `int`) is outside this rule and wrong on the corpus's `X-lenwrap-*` rows;
      a truncated varint, length or fixed field -> ERR_TRUNCATED; a varint longer than 10
      bytes -> ERR_MALFORMED;
    * nesting: a message more than `recursion_limit` levels below the root -> ERR_DEPTH;
      nested groups likewise;
    * `string` fields are validated as UTF-8 under utf8="reject" (the default, proto3's
      rule) -> ERR_TRANSCODE; "lossy" substitutes U+FFFD and is the one alternative;
    * the first error stops the decode; NOTHING is delivered after it (no flush, no apply,
      no record), and the host's output object is UNSPECIFIED and discarded (R-G6);
    * a host that reports failure (`ak_fail`, or a negative token) stops the decode at the
      next check, which follows every upcall (ABI v1 section 5, R-D6).

ABI LAYOUT (derived here once; every language's declaration is rendered from it)
----------
    presence_bits(m)        {field: bit} -- a singular message child and every explicit
                            field; bit order = tag order
    group_fields(m, enc)    [(member, abi type)] of `ak_efix_M` (enc) / `ak_dfix_M`; the
                            `ak_ufix_M` group is the e-group with child groups swapped for
                            u-groups plus a trailing `unknown: ak_blob`; every group ends
                            with `presence: u32`
    loop_slots(p, name)     [(path, field)] the vtable slots, INCLUDING those of inlined
                            children (ABI v1 section 6)
    slot_elem(f)            (decode element type, encode element type) of a slot's run
    vtable_messages(p), element_types(p), oneof_message_members(p), abi_order_topo(p)
    direct_fields(p, name), check_direct(p, root)     ABI v1 section 8 and its refusals
    check_expressible(p, root)                        a recursive message has no finite
                            group (a group inlines its whole singular subtree), so it is
                            REFUSED at generator time rather than emitted wrong
    The abi type vocabulary: i32 i64 u8 (bool) u32 (fixed32, oneof case) f64, ak_str
    (encode blob), ak_span (decode blob), ak_blob (unknown bag), ak_efix_M / ak_dfix_M /
    ak_ufix_M (a nested group). A backend maps each to its own spelling and never adds a
    member, reorders one, or re-derives a layout.
    plan.rpc: the RPC structs (`ak_bytes`, `ak_completion`, `ak_client_opts`) and the RPC
    prototypes, in the same vocabulary plus u64/usize and pointer forms (R-G5).
    plan.lifecycle: `ak_init` and what a binding must do with it (R-G7). EVERY binding a
    backend renders calls `ak_init` before its first codec or RPC call, treats AK_OK and
    AK_ALREADY_INITIALIZED as success, and fails loudly otherwise; a gate builds the core
    WITH `init-guard`, so a binding that skips it gets AK_ERR_UNINITIALIZED on its first
    call instead of passing silently (the C# slice's binding never called it, and every
    gate passed because the core's default build does not check).

What a backend MAY decide
-------------------------
    syntax and naming; how a dispatch is expressed (a `match`, a `switch`, a table); how a
    value reaches the codec (a group the host filled, a facade object read directly);
    buffer strategy (learned-width prefixes, arenas, chunk sizes); inlining; which error
    code constant spells a plan error; how it batches deliveries across the boundary.

What a backend MAY NOT decide
-----------------------------
    to add, drop or reorder a write; a presence test; which (number, wire) pairs a field
    accepts; merge versus replace; the unknown-field behaviour; the UTF-8 policy; the
    depth limit; what is delivered after an error; a layout member, its type or its
    order. A backend that needs a rule this module does not state adds it HERE, through
    the aggregating session (`CLAUDE.md`, changes to existing core behaviour), and a shape
    a backend cannot render must RAISE, never be skipped (the D12 lesson: a walker that
    silently excluded oneof members emitted a complete-looking codec that ignored them).
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import ir as IR  # noqa: E402  the ONE front end; only this module imports it

VARINT, I64, LEN, SGROUP, EGROUP, I32 = 0, 1, 2, 3, 4, 5

KIND_WIRE = {"int32": VARINT, "int64": VARINT, "bool": VARINT, "enum": VARINT,
             "double": I64, "fixed32": I32,
             "string": LEN, "bytes": LEN, "message": LEN}

VALUE_CODECS = {"int32": "varint_i32", "enum": "varint_i32", "int64": "varint_i64",
                "bool": "varint_bool", "double": "fixed64_f64", "fixed32": "fixed32_u32"}

# The ABI's scalar vocabulary for a group member of each kind (see ABI LAYOUT). Kept under
# its historical name too, because the cpp, java and python generators import it from
# `rust_abi` until they are ported to plans (WP5 migration steps 2 to 5).
ABI_SCALAR = {"int32": "i32", "int64": "i64", "bool": "u8", "double": "f64", "enum": "i32",
              "fixed32": "u32"}
CSCALAR = ABI_SCALAR


class Options:
    """The generator options. Each is a behaviour the owner has not decided, so the
    generator carries every value and the campaign measures them.

    unknown   "drop"   unknown fields are skipped;
              "retain" unknown fields are captured and re-emitted after the known ones;
              "both"   (the C ABI core's default) both families are emitted and the host
                       picks per call: `ak_encode_*`/`ak_decode_*` without capture, and
                       `ak_uencode_*`/`ak_decode_*` with the `unknown` slots set. Owner
                       position 6 / decision D4: both behaviours exist and are measured.
    utf8      "reject" (default, proto3) or "lossy" (U+FFFD substitution), on decode.
              Encode never validates: a host string type carries the invariant, and a
              converting transcoder refuses what it cannot encode (ABI v1 decision 3).
    recursion_limit   message nesting below the root beyond which a decode is refused.
    """

    def __init__(self, unknown="both", utf8="reject", recursion_limit=100):
        if unknown not in ("drop", "retain", "both"):
            raise ValueError("unknown must be drop, retain or both, not %r" % unknown)
        if utf8 not in ("reject", "lossy"):
            raise ValueError("utf8 must be reject or lossy, not %r" % utf8)
        self.unknown = unknown
        self.utf8 = utf8
        self.recursion_limit = recursion_limit

    def with_unknown(self, mode):
        return Options(mode, self.utf8, self.recursion_limit)

    @property
    def retain(self):
        return self.unknown in ("retain", "both")

    def __repr__(self):
        return "Options(unknown=%r, utf8=%r, recursion_limit=%d)" % (
            self.unknown, self.utf8, self.recursion_limit)


class EncStep:
    __slots__ = ("op", "field", "tag", "oneof")

    def __init__(self, op, field=None, oneof=None):
        self.op = op
        self.field = field
        self.tag = field.tag if field is not None else None
        self.oneof = oneof

    def __repr__(self):
        return "<enc %s %s>" % (self.op, self.field.name if self.field else "")


class DecAction:
    __slots__ = ("op", "field", "read", "oneof")

    def __init__(self, op, field, read=None, oneof=None):
        self.op = op
        self.field = field
        self.read = read
        self.oneof = oneof

    def __repr__(self):
        return "<dec %s %s %s>" % (self.op, self.field.name, self.read or "")


class FieldPlan:
    """One field: the descriptor's facts, plus the rules resolved for it."""

    def __init__(self, f, recursive):
        # The descriptor, carried so a backend never reaches for the IR.
        self.name = f.name
        self.owner = f.owner
        self.tag = f.tag
        self.kind = f.kind
        self.card = f.card
        self.of = f.of
        self.entry = f.entry
        self.explicit = f.explicit
        self.oneof = f.oneof
        self.direct = f.direct
        self.value_rule = f.value_rule
        self.adapter_site = f.adapter_site
        self.raw = f.raw
        # The rules.
        self.recursive = recursive
        if self.card in ("packed", "map"):
            self.wire = LEN
        else:
            if self.kind not in KIND_WIRE:
                raise NotImplementedError("no wire type for kind %r (%s.%s)"
                                          % (self.kind, self.owner, self.name))
            self.wire = KIND_WIRE[self.kind]
        self.value = VALUE_CODECS.get(self.kind) if self.card != "map" else None
        self.utf8 = self.kind == "string"
        if self.oneof:
            self.presence = "oneof"
        elif self.card != "singular":
            self.presence = "repeated"
        elif self.direct:
            self.presence = "direct"
        elif self.kind == "message":
            self.presence = "message"
        elif self.explicit:
            self.presence = "explicit"
        else:
            self.presence = "implicit"

    @property
    def is_scalar_leaf(self):
        return self.kind in VALUE_CODECS

    @property
    def is_blob(self):
        return self.kind in ("string", "bytes")

    @property
    def elem_wire(self):
        """The wire type ONE element of a packed field has in its unpacked form."""
        return KIND_WIRE[self.kind]

    def __repr__(self):
        return "<%s.%s tag=%d %s %s>" % (self.owner, self.name, self.tag, self.kind, self.card)


class MessagePlan:
    def __init__(self, m, plan):
        self.name = m.name
        self.synthetic = m.synthetic
        self.raw = m.raw
        self.leaf = m.leaf
        self.recursive = plan._reaches(m.name, m.name)
        self.fields = [FieldPlan(f, f.kind == "message" and plan._reaches(f.of, m.name))
                       for f in m.fields]                      # m.fields is tag-sorted
        assert [f.tag for f in self.fields] == sorted(f.tag for f in self.fields)
        self.plain = [f for f in self.fields if not f.oneof]
        self.oneofs = {}
        for f in self.fields:
            if f.oneof:
                self.oneofs.setdefault(f.oneof, []).append(f)
        self.encode = []
        self.oneof_checks = [(o, [g.tag for g in ms]) for o, ms in self.oneofs.items()]
        self.decode = {}
        for f in self.fields:
            self._lower(f)
        if plan.options.retain:
            self.encode.append(EncStep("unknown_tail"))

    # ---- one field -> its encode step and its decode actions -------------------------
    def _lower(self, f):
        enc, dec = self.encode, self.decode

        def put(key, action):
            if key in dec:
                raise AssertionError("%s: two decode actions for %r" % (self.name, key))
            dec[key] = action

        if f.oneof:
            enc.append(EncStep("oneof_member", f, oneof=f.oneof))
            put((f.tag, f.wire), DecAction("oneof_set", f, f.value, oneof=f.oneof))
            return
        if f.card == "singular":
            if f.kind == "message":
                enc.append(EncStep("child", f))
                put((f.tag, LEN), DecAction("merge_child", f))
            elif f.is_blob:
                enc.append(EncStep("blob", f))
                put((f.tag, LEN), DecAction("set_blob", f))
            else:
                enc.append(EncStep("scalar", f))
                put((f.tag, f.wire), DecAction("set_scalar", f, f.value))
        elif f.card == "packed":
            enc.append(EncStep("packed", f))
            put((f.tag, LEN), DecAction("packed_run", f, f.value))
            put((f.tag, f.elem_wire), DecAction("packed_one", f, f.value))
        elif f.card == "repeated" and f.kind == "message":
            enc.append(EncStep("repeated_message", f))
            put((f.tag, LEN), DecAction("append_message", f))
        elif f.card == "repeated" and f.is_blob:
            enc.append(EncStep("repeated_blob", f))
            put((f.tag, LEN), DecAction("append_blob", f))
        elif f.card == "map":
            self._lower_map(f)
        else:
            raise NotImplementedError("no plan for %s.%s (%s %s)"
                                      % (self.name, f.name, f.card, f.kind))

    def _lower_map(self, f):
        """A map is a repeated field of its pair message (ABI v1 section 11): the entry's
        own plan writes and reads key and value. On the key-presence rule: FIX-PLAN WP5
        lists "map key always written" (R-E4). Stated literally it is the hybrid "key
        always, empty value omitted", and on `E-map-entry-empty` (both empty) that writes
        `0A 02 0A 00`, which is NOT among the corpus's accepted encodings -- the corpus
        accepts the canonical form (both omitted when empty, what the payload manifest
        hashes and what this layer states) or "key and value always written". The rule
        is therefore stated once, here, as the canonical form, and the divergence is
        reported to the aggregating session rather than silently resolved either way."""
        self.encode.append(EncStep("map", f))
        self.decode[(f.tag, LEN)] = DecAction("map_entry", f)

    def __repr__(self):
        return "<plan %s leaf=%s%s>" % (self.name, self.leaf, " synthetic" if self.synthetic else "")


class Plan:
    def __init__(self, ir, options, source="ffi/schema/shapes.json"):
        self.options = options
        self.source = source
        self._ir = ir
        self.roots = list(ir.roots)
        self._ir_messages = ir.messages
        self.enums = dict(ir.enums)
        self.enum_order = list(ir.enum_order)
        self.order = list(ir.order)
        self.abi_order = list(ir.abi_order)
        self.messages = {}
        for name in ir.abi_order:
            self.messages[name] = MessagePlan(ir.messages[name], self)
        self.rpc = RPC
        # The description's payload definitions (value rules for payload builders), carried
        # as data so a payload builder need not reach for the schema either.
        self.payloads = dict(getattr(ir, "schema", {}).get("payloads", {}))
        self.lifecycle = LIFECYCLE

    def _reaches(self, frm, to, seen=None):
        """Can a message-typed edge path lead from `frm` to `to` (message fields only)?"""
        seen = seen or set()
        if frm in seen:
            return False
        seen.add(frm)
        m = self._ir_messages[frm]
        for f in m.fields:
            if f.kind == "message":
                if f.of == to or self._reaches(f.of, to, seen):
                    return True
        return False

    def msg(self, name):
        return self.messages[name]


def lower(ir, options=None, source=None):
    return Plan(ir, options or Options(), source or getattr(ir, "source", "ffi/schema/shapes.json"))


def relower(p, options):
    """The same description and roots, lowered with other options (lowering is pure)."""
    return Plan(p._ir, options, p.source)


def as_plan(x, options=None):
    """What a backend calls on its argument: a Plan passes through; an Ir -- which the
    cpp and java generators still hand to `rust_abi` until they are ported -- is lowered
    with the default options, so their `--check` keeps producing the committed text."""
    if isinstance(x, Plan):
        return x
    return lower(x, options)


def load(roots, options=None):
    return lower(IR.load(roots), options)


def load_schema(schema, roots, options=None, source="an in-memory description"):
    """Lower a description given as data (a test that plants a shape the real one lacks,
    e.g. `poc/rust/gen/check_direct.py`), without the caller touching the IR."""
    return lower(IR.Ir(schema, roots), options, source)


def shapes_schema():
    """The shapes.json description as data, for such a test to copy and modify."""
    return IR.S.load()


CORPUS_SOURCE = "ffi/corpus/corpus.json (reader view) over ffi/schema/shapes.json"


def load_corpus(roots=None, options=None):
    return lower(IR.load_corpus(roots), options, CORPUS_SOURCE)


def corpus_messages():
    """Every message of the corpus reader schema, in declaration order."""
    return list(IR.corpus_schema()["messages"])


# =================================================================== the ABI layout
#
# Written against the duck type both a Plan and an Ir satisfy (`.msg(name)` returning an
# object with `.plain`, `.oneofs`, `.fields`, `.leaf`, `.synthetic`), because the cpp,
# java and python generators call these through `rust_abi` with IR objects until they are
# ported. The rule is the same function either way, which is the point.

def presence_bits(m):
    """One bit per field whose presence its value cannot carry: a singular message child,
    and an explicit-presence scalar or string. Bit order is tag order."""
    bits = {}
    for f in m.plain:
        if f.oneof:
            continue
        if (f.kind == "message" and f.card == "singular") or f.explicit:
            bits[f.name] = len(bits)
    return bits


def group_fields(m, enc):
    """The group's members in tag order, oneofs after the plain fields: a oneof becomes a
    `<name>_case` discriminant carrying the ACTIVE MEMBER'S TAG (0 = none), plus every
    member inlined beside it. Not a union: a union's layout depends on which member is
    largest, which a host reproducing offsets by hand can get wrong silently (ABI v1
    section 6)."""
    out = []
    for f in m.plain:
        if f.oneof or f.card != "singular":
            continue
        if f.kind in ("string", "bytes"):
            out.append((f.name, "ak_str" if enc else "ak_span"))
        elif f.kind == "message":
            out.append((f.name, "ak_%sfix_%s" % ("e" if enc else "d", f.of)))
        else:
            out.append((f.name, ABI_SCALAR[f.kind]))
    for oname, members in m.oneofs.items():
        out.append(("%s_case" % oname, "u32"))
        for g in members:
            n = "%s_%s" % (oname, g.name)
            if g.kind in ("string", "bytes"):
                out.append((n, "ak_str" if enc else "ak_span"))
            elif g.kind == "message":
                out.append((n, "ak_%sfix_%s" % ("e" if enc else "d", g.of)))
            else:
                out.append((n, ABI_SCALAR[g.kind]))
    return out


def ugroup_fields(m):
    """`ak_ufix_M`: the encode group with the unknown-field bag (ABI v1 decision 11). A
    nested message needs its OWN bag, so a child slot is the child's u-group."""
    fields = [(fn, ty.replace("ak_efix_", "ak_ufix_")) for fn, ty in group_fields(m, True)]
    return fields + [("unknown", "ak_blob")]


def elem_type(f):
    if f.card == "map":
        return f.entry
    if f.kind == "message":
        return f.of
    return None


def slot_elem(f):
    """What one element of a loop slot looks like in a run: (decode type, encode type)."""
    if f.card == "map":
        return "ak_dfix_%s" % f.entry, "ak_efix_%s" % f.entry
    if f.kind in ("string", "bytes"):
        return "ak_span", "ak_str"
    if f.card == "packed":
        return ABI_SCALAR[f.kind], ABI_SCALAR[f.kind]
    if f.kind == "message":
        return "ak_dfix_%s" % f.of, "ak_efix_%s" % f.of
    raise NotImplementedError("slot %s %s" % (f.card, f.kind))


def loop_slots(p, name, prefix=()):
    """Every field of `name` that needs a vtable slot, INCLUDING the ones inside a
    singular message child that the encode group inlines (ABI v1 section 6: "A repeated
    or map field inside an inlined child keeps its loop slot and is reached through the
    parent"). [(path tuple, field)] in tag order, depth first."""
    out = []
    for f in p.msg(name).plain:
        if f.oneof:
            continue
        if f.card in ("repeated", "packed", "map"):
            out.append((prefix + (f.name,), f))
        elif f.kind == "message" and f.card == "singular":
            out.extend(loop_slots(p, f.of, prefix + (f.name,)))
    return out


def slot_name(path):
    return "_".join(path)


def vtable_messages(p):
    """Every root, and every element type of a repeated-message or map slot. A message
    that only ever appears as an inlined child has no vtable."""
    out = list(p.roots)
    for name in p.abi_order:
        for _, f in loop_slots(p, name):
            et = elem_type(f)
            if et and et not in out:
                out.append(et)
    return out


def element_types(p):
    out = set()
    for name in p.abi_order:
        for _, f in loop_slots(p, name):
            et = elem_type(f)
            if et:
                out.add(et)
    return out


def oneof_message_members(p):
    """A message-typed oneof member is reached through a group codec of its own rather
    than inlined: a oneof writes one member, and inlining all of them would duplicate
    every member's walk into every arm."""
    out = set()
    for name in p.abi_order:
        for _, members in p.msg(name).oneofs.items():
            for g in members:
                if g.kind == "message":
                    out.add(g.of)
    return out


def abi_order_topo(p):
    """`abi_order`, re-sorted so a group is declared after every group it inlines (C
    needs it; a run-time layout table must use the same order)."""
    out = []
    seen = set()

    def visit(name):
        if name in seen:
            return
        seen.add(name)
        for f in p.msg(name).fields:
            if f.kind == "message":
                visit(f.of)
            elif f.kind == "map":
                visit(f.entry)
        out.append(name)

    for name in p.abi_order:
        visit(name)
    return out


def direct_fields(p, name, prefix=(), seen=None):
    """Every direct-argument field ANYWHERE in the tree rooted at `name`."""
    seen = seen or set()
    if name in seen:
        return []
    seen = seen | {name}
    out = []
    for f in p.msg(name).fields:
        if f.direct:
            out.append((prefix + (f.name,), f))
        elif f.kind == "message":
            out.extend(direct_fields(p, f.of, prefix + (f.name,), seen))
    return out


def check_direct(p, root):
    """ABI v1 section 8's two generator-time refusals: a direct field in a tree that also
    needs a reverse call (a pinned buffer and an upcall are mutually exclusive), and more
    than one direct field in one tree."""
    ds = direct_fields(p, root)
    if not ds:
        return
    slots = list(loop_slots(p, root))
    for n in p.messages if isinstance(p, Plan) else p.messages:
        if n == root:
            continue
        if direct_fields(p, n):
            slots.extend(loop_slots(p, n))
    if slots:
        raise NotImplementedError(
            "REFUSED: %s declares a direct-argument field (%s) and also needs a reverse call "
            "for %s. ABI v1 section 8: a direct argument exists so the host can pin its "
            "buffer across the call, and a critical section and an upcall are mutually "
            "exclusive, so no host can honour both."
            % (root, slot_name(ds[0][0]), ", ".join(slot_name(q) for q, _ in slots)))
    if len(ds) > 1:
        raise NotImplementedError(
            "REFUSED: %s declares %d direct-argument fields (%s). ABI v1 section 8 builds the "
            "path for ONE field of one root message and nothing tests it otherwise."
            % (root, len(ds), ", ".join(slot_name(q) for q, _ in ds)))


class NotExpressible(NotImplementedError):
    pass


def check_expressible(p, root):
    """ABI v1 section 6: a group carries the WHOLE singular subtree of its message, every
    singular child inlined by value. A message that can contain itself has no finite
    group, so it cannot cross this ABI. Refused here, by name, rather than recursed into
    until the generator's stack gives out or emitted as a type C cannot declare. The
    corpus's `Nest` is the one instance; it runs on the core-native control only, and the
    rows rooted at it are reported as outside the ABI, never silently dropped."""
    seen = set()

    def walk(name, trail):
        m = p.msg(name)
        for f in m.fields:
            if f.kind != "message":
                continue
            if f.of in trail:
                raise NotExpressible(
                    "REFUSED: %s reaches %s again through %s.%s. ABI v1 section 6 inlines "
                    "every singular child into its parent's group, so a recursive message "
                    "has no finite group and cannot cross the C ABI."
                    % (root, f.of, name, f.name))
            if f.of not in seen:
                walk(f.of, trail | {f.of})
        seen.add(name)

    walk(root, {root})
    for _, f in loop_slots(p, root):
        et = elem_type(f)
        if et:
            walk(et, {et})


def expressible_roots(p_or_names, p=None):
    """The subset of candidate roots the ABI can carry, and the refusals for the rest."""
    ok, refused = [], {}
    for r in p_or_names:
        try:
            check_expressible(p, r)
            ok.append(r)
        except NotExpressible as e:
            refused[r] = str(e)
    return ok, refused


# =================================================================== the RPC half (R-G5)
#
# ABI v1 section 9. The core had these declared twice in Rust (ak-abi's host-facing
# declaration and ak-core's definition) with one hand-written offset check for one of
# them, `ak_queue_next`'s timeout was `i32` on one side and `u64` on the other, and the
# cpp slice declares them a third time by hand. They are stated once, here; the Rust
# backend renders the declaration AND a compile-time assertion in the core that each
# definition has exactly this signature and layout.

class RpcAbi:
    handles = ["ak_runtime", "ak_client", "ak_call", "ak_queue"]

    structs = [
        ("ak_bytes",
         "A byte range the core owns until the host releases it with `ak_bytes_free`.",
         [("ptr", "*const u8", ""),
          ("len", "usize", ""),
          ("owner", "*mut void", "The core's handle on the allocation. The host passes it "
                                 "back and does not read it.")]),
        ("ak_completion",
         "What a completion carries. Released with `ak_bytes_free`, as the blocking "
         "delivery's bytes are, so a host has one release path whichever delivery it takes.",
         [("tag", "u64", ""),
          ("status", "i32", ""),
          ("bytes", "ak_bytes", "")]),
        ("ak_client_opts",
         "The transport settings ArmoniK pins. The stream and the connection window are "
         "separate settings on tonic/hyper, so both are here. Zero means \"the stack's\".",
         [("stream_window", "u32", "SETTINGS_INITIAL_WINDOW_SIZE per stream; 0 = default. ArmoniK: 4 MiB."),
          ("connection_window", "u32", "The connection-level window, a SEPARATE setting; 0 = default."),
          ("adaptive_window", "i32", "1 on, 0 off, -1 default (off). Overrides both windows when on."),
          ("max_recv_message", "u32", "Largest message accepted, bytes; 0 = default."),
          ("max_send_message", "u32", "Largest message sent, bytes; 0 = default."),
          ("tcp_nagle", "i32", "ArmoniK's tcp_nagle_algorithm: 1 Nagle on, 0 off, -1 default. "
                               "ArmoniK ships it OFF.")]),
    ]

    callbacks = [
        ("ak_completion_cb", [("user_data", "*mut void"), ("comp", "*mut ak_completion")], None,
         "Called ONCE per call, on a thread the core owns."),
    ]

    constants = [
        ("AK_QUEUE_OK", "i32", 0, "`ak_queue_next` returned a completion."),
        ("AK_QUEUE_TIMEOUT", "i32", 1, "The timeout expired with no completion. Not an error."),
        ("AK_QUEUE_SHUTDOWN", "i32", 2, "The queue is shutting down and is drained."),
    ]

    # (name, [(param, type)], return type or None, doc)
    functions = [
        ("ak_runtime_new", [("worker_threads", "u32")], "*mut ak_runtime", ""),
        ("ak_runtime_destroy", [("r", "*mut ak_runtime")], None, ""),
        ("ak_client_new", [("r", "*mut ak_runtime"), ("uri", "*const u8"), ("uri_len", "usize")],
         "*mut ak_client", ""),
        ("ak_client_new_opts", [("r", "*mut ak_runtime"), ("uri", "*const u8"), ("uri_len", "usize"),
                                ("opts", "*const ak_client_opts")], "*mut ak_client",
         "A client with the transport pinned. NULL options = `ak_client_new`."),
        ("ak_client_destroy", [("c", "*mut ak_client")], None, ""),
        ("ak_call_unary", [("c", "*mut ak_client"), ("path", "*const u8"), ("path_len", "usize"),
                           ("req", "*const u8"), ("req_len", "usize"), ("out", "*mut ak_bytes")],
         "i32", "Blocking delivery: one crossing in, `ak_bytes_free` the only other."),
        ("ak_bytes_free", [("b", "*mut ak_bytes")], None, ""),
        ("ak_call_unary_cb", [("c", "*mut ak_client"), ("path", "*const u8"), ("path_len", "usize"),
                              ("req", "*const u8"), ("req_len", "usize"), ("cb", "ak_completion_cb"),
                              ("user_data", "*mut void"), ("tag", "u64")], "*mut ak_call",
         "Callback delivery: 2 forward crossings and 1 reverse."),
        ("ak_call_unary_q", [("c", "*mut ak_client"), ("path", "*const u8"), ("path_len", "usize"),
                             ("req", "*const u8"), ("req_len", "usize"), ("q", "*mut ak_queue"),
                             ("tag", "u64")], "*mut ak_call",
         "Completion-queue delivery: no upcall. 3 forward crossings and 0 reverse."),
        ("ak_queue_new", [], "*mut ak_queue", ""),
        ("ak_queue_next", [("q", "*mut ak_queue"), ("out", "*mut ak_completion"), ("timeout_ms", "u64")],
         "i32", "Wait up to `timeout_ms` for one completion. R-G5: `u64`, as the core defines it."),
        ("ak_queue_shutdown", [("q", "*mut ak_queue")], None, ""),
        ("ak_queue_destroy", [("q", "*mut ak_queue")], None, ""),
        ("ak_call_cancel", [("h", "*mut ak_call")], None, ""),
        ("ak_call_destroy", [("h", "*mut ak_call")], None, ""),
    ]


RPC = RpcAbi()


# =================================================================== the lifecycle (R-G7)

class Lifecycle:
    """ABI v1 section 3, as every binding must render it. `ak_init` is not optional and not
    the harness's business: a binding that does not call it is a binding whose first codec
    call returns AK_ERR_UNINITIALIZED on a core built as specified (`init-guard`)."""
    init = ("ak_init", [("opts", "*const ak_init_opts"), ("err", "*mut ak_err")], "i32")
    opts_struct = ("ak_init_opts", [("abi_version", "u32"), ("flags", "u32"),
                                    ("log", "ak_log_fn (nullable)"), ("log_ctx", "*mut void")])
    success = ("AK_OK", "AK_ALREADY_INITIALIZED")
    # What a codec binding passes: no crypto provider in a codec build, the host keeps its
    # own panic hook. A binding may add AK_INIT_OWN_LOGGING; it may not skip the call.
    default_flags = ("AK_INIT_NO_CRYPTO", "AK_INIT_NO_PANIC_HOOK")
    required_before = "every codec and RPC entry point"
    gate_feature = "init-guard"
    # WP5 step 6 (R-G13): the VALUES the names above stand for, stated here rather than
    # restated by each header renderer. They equal ABI v1 section 3.
    flag_values = (("AK_INIT_OWN_LOGGING", 1 << 0), ("AK_INIT_NO_PANIC_HOOK", 1 << 1),
                   ("AK_INIT_NO_CRYPTO", 1 << 2))


LIFECYCLE = Lifecycle()



# =================================================================== the fixed ABI (R-G13)
#
# WP5 step 6. Everything of ABI v1 that does not depend on a description: section 5's
# error codes, section 4's vocabulary, the fixed entry points of sections 3, 4, 5, 7.1 and
# 10, the counting surfaces. Before this every header renderer (the C header twice, the
# C# declarations, ak-abi's Rust) restated it as fixed text. It is data here; each backend
# renders it and adds nothing.
#
# The type vocabulary (the same as the layout's, plus): `void` / `char` behind pointers;
# `*const T`, `*mut T`, `*mut *const u8`; a function-pointer type by NAME (the `fn_types`
# below); `T?` a NULLABLE function pointer (Rust `Option<T>`, C a plain pointer); and one
# anonymous function type, `fn(u64)->u64`, the parameter of `ak_noop_reverse`.

class FixedAbi:
    abi_version = 1

    # (name, value, meaning). ABI v1 section 5.
    codes = [
        ("AK_OK", 0, "success"),
        ("AK_ALREADY_INITIALIZED", 1, "success: ak_init called again with the same options"),
        ("AK_ERR_HOST", -1, "the host reported through ak_fail"),
        ("AK_ERR_MALFORMED", -2, "invalid wire"),
        ("AK_ERR_TRUNCATED", -3, "the input ended inside a value"),
        ("AK_ERR_DEPTH", -4, "the decode recursion limit"),
        ("AK_ERR_LIMIT", -5, "a size limit"),
        ("AK_ERR_TRANSCODE", -6, "the transcoder refused its input"),
        ("AK_ERR_CAPACITY", -7, "a transcoder wrote past the capacity given"),
        ("AK_ERR_INVALID_STATE", -8, "e.g. two concurrent recv on one call"),
        ("AK_ERR_PANIC", -9, "a caught Rust panic"),
        ("AK_ERR_UNINITIALIZED", -10, "ak_init was not called"),
        ("AK_ERR_ABI", -11, "version or group-layout mismatch"),
    ]
    # ak_err.detail values (ABI v1 section 3).
    details = [("AK_DETAIL_NONE", 0), ("AK_DETAIL_ABI_MISMATCH", 1),
               ("AK_DETAIL_OPTS_DIFFER", 2), ("AK_DETAIL_NULL_ARG", 3)]

    handles = ["ak_enc_ctx", "ak_dec_ctx"]

    # (name, [(param, type)], return type or None, doc)
    fn_types = [
        ("ak_grow_fn", [("sink", "*mut void"), ("want", "i32"), ("dst", "*mut *mut u8"),
                        ("cap", "*mut i32")], "i32",
         "ABI v1 section 4: the transcoder's growth callback; may move the buffer."),
        ("ak_transcode_fn", [("src", "*const void"), ("len", "usize"), ("dst", "*mut u8"),
                             ("cap", "i32"), ("grow", "ak_grow_fn"), ("sink", "*mut void")], "i32",
         "ABI v1 section 4: source code units -> UTF-8 into dst; returns bytes written or an error."),
        ("ak_log_fn", [("ctx", "*mut void"), ("level", "u32"), ("msg", "*const u8"),
                       ("msg_len", "usize")], None,
         "ABI v1 section 3: the host's log sink."),
        ("ak_loop_f", [("ctx", "*mut ak_enc_ctx"), ("obj", "*const void"), ("token", "i64")], "i32",
         "ABI v1 section 6: a host-driven loop over one repeated/packed/map field."),
        ("ak_unk_f", [("ctx", "*mut ak_dec_ctx"), ("obj", "*mut void"),
                      ("spans", "*const ak_uspan"), ("n", "i32")], None,
         "Decision 11 candidate: captured unknown-field runs, delivered by token."),
    ]

    # (name, doc, [(member, type)]). Laid out repr(C) / C order, members as listed.
    structs = [
        ("ak_str", "ABI v1 section 4, encode: a blob as DATA in the group. `len` counts SOURCE "
                   "code units; `tc == NULL` means the field is ABSENT.",
         [("data", "*const void"), ("len", "usize"), ("tc", "ak_transcode_fn?")]),
        ("ak_span", "ABI v1 section 4, decode: an OFFSET into the buffer the host handed in.",
         [("off", "u32"), ("len", "u32"), ("coder", "u32")]),
        ("ak_blob", "Decision 11's unknown-field bag: two words, raw tag-and-value runs.",
         [("data", "*const void"), ("len", "usize")]),
        ("ak_uspan", "One captured unknown run: which object (a token) and where in the input.",
         [("token", "i64"), ("off", "u32"), ("len", "u32")]),
        ("ak_err", "ak_init's out-parameter (ABI v1 section 3/5): a code and a detail.",
         [("code", "i32"), ("detail", "u32")]),
        ("ak_init_opts", "ak_init's options (ABI v1 section 3).",
         [("abi_version", "u32"), ("flags", "u32"), ("log", "ak_log_fn?"),
          ("log_ctx", "*mut void")]),
        ("AkCounters", "README R5: boundary-call counts, counted in the CORE (zero unless the "
                       "core is a counting build).",
         [("forward", "u64"), ("reverse", "u64"), ("transcode", "u64"),
          ("prefix_moves", "u64"), ("prefix_bytes", "u64"), ("grows", "u64")]),
        ("ak_bdr_rec", "ABI v1 section 7.1: one pull record header, 24 bytes; the payload "
                       "follows, padded to 8.",
         [("op", "u32"), ("slot", "u32"), ("token", "i64"), ("n", "u32"), ("bytes", "u32")]),
    ]
    # The sizes a renderer asserts (LP64 / 64-bit hosts, which is every host this ships to).
    sizes = {"ak_str": 24, "ak_span": 12, "ak_blob": 16, "ak_uspan": 16, "ak_err": 8,
             "ak_init_opts": 24, "AkCounters": 48, "ak_bdr_rec": 24}

    # (name, type, value, doc). `AK_STR_DIRECT` is a pointer VALUE, `AK_TOKEN_ROOT` an i64.
    constants = [
        ("AK_STR_DIRECT", "*const void", 1, "ABI v1 section 8: ak_str.data meaning 'a direct "
                                             "argument of the call'."),
        ("AK_TOKEN_ROOT", "i64", -1, "a token naming the root object itself"),
        ("AK_BDR_APPLY", "u32", 1, "pull record: the root group (last record)"),
        ("AK_BDR_ADD", "u32", 2, "pull record: a batched run"),
        ("AK_BDR_NEW", "u32", 3, "pull record: a non-leaf element begins (minted token)"),
        ("AK_BDR_APPLY_ELEM", "u32", 4, "pull record: a non-leaf element's group"),
        ("AK_BDR_MIN_CHUNK", "usize", 32 * 1024 + 24, "the smallest drain chunk that holds any record"),
    ]

    # The packed-run symbols: one per HOST array layout (ABI v1 section 6).
    run_types = ("i32", "i64", "f64", "u8")

    # (group, name, [(param, type)], return type or None, doc). `group` lets a renderer lay
    # them out in sections; every name here is exported by every core build.
    functions = [
        ("lifecycle", "ak_init", [("opts", "*const ak_init_opts"), ("err", "*mut ak_err")], "i32",
         "Once per process, before anything else (plan.lifecycle)."),
        ("lifecycle", "ak_initialized", [], "i32", "1 once ak_init has returned successfully."),
        ("lifecycle", "ak_build_id", [], "*const char", "A NUL-terminated build id."),
        ("lifecycle", "ak_log_test", [("level", "u32"), ("msg", "*const u8"), ("len", "usize")], "i32",
         "Test hook: one line through the installed log bridge."),
        ("lifecycle", "ak_panic_test", [], None, "Test hook: a panic inside the core."),
        ("context", "ak_abi_version", [], "u32", ""),
        ("context", "ak_enc_ctx_new", [], "*mut ak_enc_ctx", ""),
        ("context", "ak_enc_ctx_free", [("ctx", "*mut ak_enc_ctx")], None, ""),
        ("context", "ak_enc_reset", [("ctx", "*mut ak_enc_ctx")], None,
         "Clears the buffer and the sticky slot."),
        ("context", "ak_enc_take", [("ctx", "*mut ak_enc_ctx"), ("ptr", "*mut *const u8"),
                                    ("len", "*mut usize")], "i32",
         "Borrow what the context has encoded, valid until the next reset; returns the status."),
        ("context", "ak_dec_ctx_new", [], "*mut ak_dec_ctx", ""),
        ("context", "ak_dec_ctx_free", [("ctx", "*mut ak_dec_ctx")], None, ""),
        ("error", "ak_fail", [("ctx", "*mut void"), ("code", "i32"), ("msg", "*const u8"),
                              ("msg_len", "u32")], None,
         "Sticky, first error wins; takes either context (ABI v1 section 5)."),
        ("error", "ak_enc_err", [("ctx", "*const ak_enc_ctx")], "i32", ""),
        ("error", "ak_dec_err", [("ctx", "*const ak_dec_ctx")], "i32", ""),
        ("error", "ak_dec_err_reset", [("ctx", "*mut ak_dec_ctx")], None, ""),
        ("transcode", "ak_tc_utf8", [], "ak_transcode_fn", "UTF-8, validate and refuse."),
        ("transcode", "ak_tc_utf8_trusted", [], "ak_transcode_fn", "UTF-8, no validation."),
        ("transcode", "ak_tc_utf8_simd", [], "ak_transcode_fn", "UTF-8, SIMD validator."),
        ("transcode", "ak_tc_bytes", [], "ak_transcode_fn", "bytes: a copy."),
        ("transcode", "ak_tc_utf16", [], "ak_transcode_fn", "UTF-16 code units -> UTF-8."),
        ("transcode", "ak_tc_latin1", [], "ak_transcode_fn", "Latin-1 bytes -> UTF-8."),
        ("counters", "ak_enc_counters", [("ctx", "*const ak_enc_ctx"), ("out", "*mut AkCounters")], None, ""),
        ("counters", "ak_enc_count_reverse", [("ctx", "*mut ak_enc_ctx")], None,
         "Counting build: the HOST reports a reverse crossing the core cannot see."),
        ("counters", "ak_enc_counters_reset", [("ctx", "*mut ak_enc_ctx")], None, ""),
        ("counters", "ak_dec_counters", [("ctx", "*const ak_dec_ctx"), ("out", "*mut AkCounters")], None, ""),
        ("counters", "ak_dec_counters_reset", [("ctx", "*mut ak_dec_ctx")], None, ""),
        ("counters", "ak_enc_site_moves", [("ctx", "*const ak_enc_ctx"), ("out", "*mut u32"),
                                           ("cap", "usize")], "usize", ""),
        ("noop", "ak_noop", [("x", "u64")], "u64", "A crossing and nothing else."),
        ("noop", "ak_noop2", [("x", "u64")], "u64", "An identical twin of ak_noop."),
        ("noop", "ak_noop_guarded", [("x", "u64")], "u64", "ak_noop with the init guard."),
        ("noop", "ak_noop_reverse", [("f", "fn(u64)->u64"), ("x", "u64")], "u64",
         "One forward and one reverse crossing."),
        ("pull", "ak_bdr_reserve", [("ctx", "*mut ak_dec_ctx"), ("bytes", "usize")], "i32", ""),
        ("pull", "ak_bdr_footprint", [("ctx", "*const ak_dec_ctx")], "usize", ""),
        ("pull", "ak_bdr_drain", [("ctx", "*mut ak_dec_ctx"), ("dst", "*mut u8"), ("cap", "usize"),
                                  ("cursor", "*mut usize")], "isize", ""),
        ("pull", "ak_bdr_ptr", [("ctx", "*mut ak_dec_ctx"), ("ptr", "*mut *const u8"),
                                ("len", "*mut usize")], "i32", ""),
        ("pull", "ak_bdr_reset", [("ctx", "*mut ak_dec_ctx")], None, ""),
        ("pull", "ak_bdr_count_forward", [("ctx", "*mut ak_dec_ctx"), ("n", "u32")], None,
         "Counting build: the host reports the forward crossings of a drain loop."),
        ("layout", "ak_layout_facts", [("out", "*mut u32"), ("cap", "usize")], "usize",
         "ABI v1 section 10: the core's own view of every group layout."),
    ]

    # The RPC counting surface (README R5 for section 9). Exported with the `rpc` feature.
    rpc_counters_struct = ("ak_rpc_counters", "RPC boundary-call counts (counting build).",
                           [("forward", "u64"), ("reverse", "u64")])
    rpc_counting = [
        ("rpc_count", "ak_rpc_counting", [], "i32", "1 if this core counts RPC crossings."),
        ("rpc_count", "ak_rpc_counters", [("out", "*mut ak_rpc_counters")], None, ""),
        ("rpc_count", "ak_rpc_counters_reset", [], None, ""),
    ]

    def run_functions(self):
        """`ak_run_<ty>` and `ak_blob_run`: fixed names, rendered from `run_types`."""
        out = [("run", "ak_blob_run", [("ctx", "*mut ak_enc_ctx"), ("elems", "*const ak_str"),
                                       ("n", "i32")], "i32",
                "A run of strings or bytes under the repeated field the codec has open.")]
        for ty in self.run_types:
            out.append(("run", "ak_run_%s" % ty, [("ctx", "*mut ak_enc_ctx"), ("p", "*const %s" % ty),
                                                   ("n", "usize")], "i32",
                        "A packed repeated scalar: the host's own array, handed over whole."))
        return out

    def all_functions(self):
        return self.functions + self.run_functions()


FIXED = FixedAbi()
BDR = {n: v for n, _t, v, _d in FIXED.constants if n.startswith("AK_BDR_")}


# =================================================================== vtables and pull records
#
# WP5 step 6 (R-G13, Java's G1). The ORDER and SHAPE of the vtable structs and the pull
# family's record slot numbering, stated once. Before this they were rendered by
# `rust_abi` and restated by `java_abi` from the same facts.

def batchable(p, f):
    """ABI v1 section 7.2: a run is batched iff its element type, if any, is a leaf."""
    et = elem_type(f)
    return not (et and not p.msg(et).leaf)


def enc_vtable(p, name):
    """`ak_evt_<name>` members, in order, as (kind, slot name, element type): kind is
    `loop` (ak_loop_f loop_<slot>), `elem` (a pointer to the element's own vtable, right
    after its loop slot, when the element has loop slots) or `reserved` (an empty vtable
    carries one pointer: an empty struct has no defined size in C)."""
    slots = loop_slots(p, name)
    if not slots:
        return [("reserved", "_reserved", None)]
    rows = []
    for path, f in slots:
        sn = slot_name(path)
        rows.append(("loop", sn, None))
        et = elem_type(f)
        if et and loop_slots(p, et):
            rows.append(("elem", sn, et))
    return rows


def dec_vtable(p, name):
    """`ak_dvt_<name>` members, in order: `apply`, `unknown`, then per loop slot: `unk`
    (the slot's elements' unknown runs, when it has an element type), then `add` (a
    batchable run) or `new`, `applyelem` and one `addinner` per inner slot of a non-leaf
    element. Each row is (kind, slot name, x): x is the element type (unk, add, new,
    applyelem), the inner FieldPlan (addinner) or None. A non-leaf element inside a non-leaf
    element is refused."""
    rows = [("apply", "", None), ("unknown", "", None)]
    for path, f in loop_slots(p, name):
        sn = slot_name(path)
        et = elem_type(f)
        if et:
            rows.append(("unk", sn, et))
        if batchable(p, f):
            rows.append(("add", sn, et))
        else:
            rows.append(("new", sn, et))
            rows.append(("applyelem", sn, et))
            for ipath, iff in loop_slots(p, et):
                iet = elem_type(iff)
                if iet and not p.msg(iet).leaf:
                    raise NotImplementedError(
                        "a non-leaf element inside a non-leaf element (%s.%s): the schema has "
                        "no instance, so this is refused rather than emitted untested"
                        % (et, slot_name(ipath)))
                rows.append(("addinner", "%s_%s" % (sn, slot_name(ipath)), iff))
    return rows


def enc_slots(p):
    """[(vtable message, slot path, field)] in vtable order, globally numbered."""
    out = []
    for name in vtable_messages(p):
        for path, f in loop_slots(p, name):
            out.append((name, path, f))
    return out


def dec_slots(p):
    """Every decode callback of a ROOT's vtable that a binding wires, as (root, kind, slot
    name, x); the unknown-field slots are omitted (a binding that captures wires them
    itself)."""
    out = []
    for name in p.roots:
        for kind, sn, x in dec_vtable(p, name):
            if kind in ("unknown", "unk"):
                continue
            out.append((name, kind, sn, x))
    return out


def pull_slot(j, k=0):
    """The pull family's record slot number: `(outer << 16) | inner`, both 1-based; the
    root group is slot 0. `j` is the 0-based loop slot of the root, `k` the 0-based inner
    slot of a non-leaf element (k = -1: the element itself, inner 0)."""
    return ((j + 1) << 16) | (k + 1) if k >= 0 else (j + 1) << 16


def pull_records(p, root):
    """[(op, slot, kind, slot name)] a pull decode of `root` can write: the numbering both
    the core (`ak_parse_*`) and every host replay use. A batchable run at the root is the
    bare index j + 1 (inner 0, outer omitted)."""
    out = [(BDR["AK_BDR_APPLY"], 0, "apply", "")]
    for j, (path, f) in enumerate(loop_slots(p, root)):
        sn = slot_name(path)
        if batchable(p, f):
            out.append((BDR["AK_BDR_ADD"], j + 1, "add", sn))
        else:
            et = elem_type(f)
            out.append((BDR["AK_BDR_NEW"], pull_slot(j, -1), "new", sn))
            out.append((BDR["AK_BDR_APPLY_ELEM"], pull_slot(j, -1), "applyelem", sn))
            for k, (ipath, _iff) in enumerate(loop_slots(p, et)):
                out.append((BDR["AK_BDR_ADD"], pull_slot(j, k), "addinner", "%s_%s" % (sn, slot_name(ipath))))
    return out
