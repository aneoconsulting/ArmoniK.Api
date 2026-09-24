"""Backend: payload construction, emitted TWICE over two object models.

`ffi/schema/emit/payloads.py` is the description of what is in a payload. This
backend re-derives it as C# that builds an object graph, and emits the same
rules against two different sinks:

  * the facade (arms `managed-encode` / `managed-decode`), and
  * `Google.Protobuf`'s own generated classes (the incumbent arm).

**Why two emitted routes and not one plus a converter.** A converter makes the
incumbent's graph a function of the facade's, so a defect in facade
construction becomes invisible: both arms encode the same wrong value and
agree. Two routes disagreeing is what caught the Rust slice's D15. They share
the RULES (one description, R1) and nothing else; the manifest is the external
oracle over both, so a defect in the shared rules is caught by the hashes
rather than by the arms agreeing with each other.

**Every path is constant-folded.** `payloads.py` threads a `path` string
through its recursion to make each value unique to its field; the call graph is
static, so this backend resolves every path at generation time and the emitted
code contains the literal. Nothing about the values changes; what changes is
that a reader can see which path a field got.
"""
import cs_names as N
from glue import Head

MODES = (None, "all_absent", "half_absent")


def desc(m):
    """A message's fields in DESCRIPTION order (the plan's are in tag order; the payload
    definitions name fields by position in the description)."""
    names = [f["name"] for f in m.raw.get("fields", [])]
    return sorted(m.fields, key=lambda f: names.index(f.name) if f.name in names else len(names))


def mode_tag(mode):
    return {None: "Full", "all_absent": "Absent", "half_absent": "Half"}[mode]


def absent(f, mode):
    """Which fields the absent-path modes remove (payloads.py `absent`).

    Independent of the element index for every rule the description has, so it
    folds at generation time. If a mode is ever added whose rule depends on the
    index this must become emitted code, and the assertion that it does not is
    that `payloads.py` takes `idx` and uses it in neither branch.
    """
    if mode == "all_absent":
        return True
    if mode == "half_absent":
        if f.adapter_site == "nested":
            return True
        if f.kind == "message" and f.of == "Timestamp" and f.tag % 2 == 0:
            return True
    return False


class Sink:
    """How one object model spells an assignment. The rules live in the emitter."""
    def typ(self, name):
        return "%s.%s" % (self.ns, name)

    def new(self, name):
        return "new %s()" % self.typ(name)

    def enum(self, ename, expr):
        raise NotImplementedError

    def bytes_(self, expr):
        raise NotImplementedError

    def set(self, var, f, expr):
        return "%s.%s = %s;" % (var, N.field(f.name), expr)

    def add(self, var, f, expr):
        return "%s.%s.Add(%s);" % (var, N.field(f.name), expr)

    def put(self, var, f, k, v):
        return "%s.%s[%s] = %s;" % (var, N.field(f.name), k, v)

    def set_oneof(self, var, m, f, expr):
        raise NotImplementedError

    def clear_absent(self, var, f):
        """What it takes to leave an explicit-presence field ABSENT.

        Nothing in both models -- a field never assigned is absent -- but the
        emitter calls it so that the absent branch is visible in the output
        rather than being an empty `if`.
        """
        return None


class FacadeSink(Sink):
    name = "Facade"
    ns = "Armonik.Ffi.Facade"

    def typ(self, name):
        return name                    # the builder lives in the facade namespace

    def new(self, name):
        return "new %s()" % name

    def enum(self, ename, expr):
        return expr                    # Values returns the facade enum already

    def bytes_(self, expr):
        return expr

    def set_oneof(self, var, m, f, expr):
        return "%s.%sCase = %s.%s; %s.%s = %s;" % (
            var, N.pascal(f.oneof), N.oneof_case_type(m.name, f.oneof), N.pascal(f.name),
            var, N.field(f.name), expr)


class GpSink(Sink):
    name = "Gp"
    ns = "Armonik.Ffi.Shapes.V1"

    def enum(self, ename, expr):
        # The numeric values come from one description, so the cast is a
        # re-tagging and not a conversion. Spelled through `int` so that a
        # future divergence in the numbers is a compile error and not a
        # silently wrong enum.
        return "(%s.%s)(int)(%s)" % (self.ns, ename, expr)

    def bytes_(self, expr):
        return "pb::ByteString.CopyFrom(%s)" % expr

    def set_oneof(self, var, m, f, expr):
        return "%s.%s = %s;" % (var, N.field(f.name), expr)      # setting a member sets the case


class Emitter:
    def __init__(self, ir, sink):
        self.ir = ir
        self.s = sink
        self.fns = {}          # (msg, path, mode, form) -> name
        self.body = []

    # ---- value expressions, one per (kind, value rule) ------------------

    def text(self, f, path, idx):
        rule = f.value_rule or "word"
        fn = {"guid": "Guid", "word": "Word", "sentence": "Sentence"}.get(rule)
        if fn is None:
            raise KeyError("no case for string value rule %r on %s" % (rule, f))
        return 'Values.%s("%s", %s)' % (fn, path, idx)

    def scalar(self, kind, path, idx):
        fn = {"int32": "ScalarI32", "int64": "ScalarI64",
              "bool": "ScalarBool", "double": "ScalarF64"}[kind]
        return 'Values.%s("%s", %s)' % (fn, path, idx)

    # ---- the recursion ---------------------------------------------------

    def fn_for(self, msg, path, mode, form):
        """Emit (once) a builder for one message at one path under one mode."""
        key = (msg.name, path, mode, form)
        if key in self.fns:
            return self.fns[key]
        name = "Build%s_%s_%s" % (form, msg.name, mode_tag(mode))
        if path != msg.name:
            name += "_" + path.replace(".", "_")
        self.fns[key] = name
        lines = []
        bulk = form == "Bulk"
        sig = "long idx, int repeats" + (", int bulkSize" if bulk else "")
        lines.append("    public static %s %s(%s)" % (self.s.typ(msg.name), name, sig))
        lines.append("    {")
        lines.append("        var m = %s;" % self.s.new(msg.name))
        fields = msg.plain if form == "Element" else desc(msg)
        if form != "Element" and msg.oneofs:
            raise KeyError(
                "%s is reached as a NESTED message and has a oneof; payloads.py's "
                "enc_message has no case for that, so neither does this backend. "
                "R1: raise rather than skip." % msg.name)
        for f in fields:
            self.emit_field(lines, msg, f, path, mode, bulk)
        if form == "Element":
            self.emit_oneof(lines, msg, path, mode)
        lines.append("        return m;")
        lines.append("    }")
        lines.append("")
        self.body.extend(lines)
        return name

    def emit_field(self, L, msg, f, parent_path, mode, bulk):
        path = "%s.%s" % (parent_path, f.name)
        pad = "        "
        if absent(f, mode):
            L.append("%s// %s: removed by mode %s" % (pad, f.name, mode))
            return

        if f.card == "map":
            L.append("%s{" % pad)
            L.append("%s    // %d entries, keys sorted: the canonical form fixes an order"
                     % (pad, f.raw.get("entries", 4)))
            L.append("%s    // protobuf does not define, and MapField writes insertion order."
                     % pad)
            L.append("%s    var keys = new string[%d]; var vals = new string[%d];"
                     % (pad, f.raw.get("entries", 4), f.raw.get("entries", 4)))
            L.append("%s    for (int k = 0; k < %d; k++)" % (pad, f.raw.get("entries", 4)))
            L.append("%s    {" % pad)
            L.append('%s        keys[k] = "k" + k.ToString("D2") + "-" + Values.Word("%s.key", idx * 31 + k);'
                     % (pad, path))
            note = "   // mode half_absent: even-index values emptied" if mode == "half_absent" else ""
            if mode == "half_absent":
                L.append('%s        vals[k] = (k %% 2 == 0) ? "" : Values.Word("%s.value", idx * 31 + k);'
                         % (pad, path))
            else:
                L.append('%s        vals[k] = Values.Word("%s.value", idx * 31 + k);' % (pad, path))
            L.append("%s    }" % pad)
            L.append("%s    Array.Sort(keys, vals, StringComparer.Ordinal);" % pad)
            L.append("%s    for (int k = 0; k < %d; k++) %s" % (
                pad, f.raw.get("entries", 4), self.s.put("m", f, "keys[k]", "vals[k]")))
            L.append("%s}%s" % (pad, note))
            return

        if f.card == "packed":
            L.append("%sfor (int j = 0; j < %d; j++) %s" % (
                pad, f.raw.get("count", 30),
                self.s.add("m", f,
                           self.s.enum(f.of, 'Values.%sAt(idx * 97 + j)' % f.of)
                           if f.kind == "enum"
                           else self.scalar(f.kind, path, "idx * 97 + j"))))
            return

        if f.card == "repeated":
            if f.kind == "string":
                L.append('%sfor (int j = 0; j < repeats; j++) %s' % (
                    pad, self.s.add("m", f, 'Values.Guid("%s", idx * 211 + j)' % path)))
                return
            # repeated message: payloads.py indexes the ELEMENT by the repeat
            # counter j, not by the parent's idx. Nothing in the payload set
            # reaches this, and it is emitted anyway because R1 forbids a
            # backend that quietly has no case for a shape.
            sub = self.ir.msg(f.of)
            fn = self.fn_for(sub, path, mode, "Nested")
            L.append("%sfor (int j = 0; j < repeats; j++) %s" % (
                pad, self.s.add("m", f, "%s(j, repeats)" % fn)))
            return

        # ---- singular -------------------------------------------------
        if f.kind == "string":
            if f.adapter_site == "plain":
                L.append("%s// the adapter's PLAIN site: Ok and Invalid BOTH flatten to the" % pad)
                L.append("%s// empty string here, so one of them must come back wrong." % pad)
                L.append("%sif (Values.AdapterState(idx) == 0) %s" % (
                    pad, self.s.set("m", f, self.text(f, path, "idx"))))
                return
            if f.explicit:
                L.append("%sif (Values.ExplicitPresent(%d, idx))" % (pad, f.tag))
                L.append("%s    %s" % (pad, self.s.set(
                    "m", f, 'Values.PresentZero(idx) ? "" : %s' % self.text(f, path, "idx"))))
                return
            L.append("%s%s" % (pad, self.s.set("m", f, self.text(f, path, "idx"))))
            return

        if f.kind == "bytes":
            src = "Values.Bulk(bulkSize)" if (f.value_rule == "bulk" and bulk) \
                else 'Values.Blob("%s", idx)' % path
            L.append("%s%s" % (pad, self.s.set("m", f, self.s.bytes_(src))))
            return

        if f.kind == "enum":
            L.append("%s%s" % (pad, self.s.set(
                "m", f, self.s.enum(f.of, "Values.%sAt(idx)" % f.of))))
            return

        if f.kind == "message":
            if f.adapter_site == "nested":
                self.emit_adapter_nested(L, msg, f, path, mode)
                return
            if f.of in ("Timestamp", "Duration"):
                pre = "Stamp" if f.of == "Timestamp" else "Dur"
                L.append("%s%s" % (pad, self.s.set("m", f, "%s(idx)" % (
                    "NewStamp" if f.of == "Timestamp" else "NewDur"))))
                return
            sub = self.ir.msg(f.of)
            fn = self.fn_for(sub, path, mode, "Bulk" if bulk else "Nested")
            args = "idx, repeats" + (", bulkSize" if bulk else "")
            L.append("%s%s" % (pad, self.s.set("m", f, "%s(%s)" % (fn, args))))
            return

        # scalar
        if f.explicit:
            L.append("%sif (Values.ExplicitPresent(%d, idx))" % (pad, f.tag))
            zero = {"bool": "false", "double": "0.0"}.get(f.kind, "0")
            L.append("%s    %s" % (pad, self.s.set(
                "m", f, "Values.PresentZero(idx) ? %s : %s" % (
                    zero, self.scalar(f.kind, path, "idx")))))
            return
        L.append("%s%s" % (pad, self.s.set("m", f, self.scalar(f.kind, path, "idx"))))

    def emit_adapter_nested(self, L, msg, f, path, mode):
        """`Output` at its nested site: three states, one of them no child at all."""
        pad = "        "
        child = self.ir.msg(f.of)
        succ = next(c for c in desc(child) if c.name == "success")
        err = next(c for c in desc(child) if c.name == "error")
        L.append("%sswitch (Values.AdapterState(idx))" % pad)
        L.append("%s{" % pad)
        L.append("%s    case 1:   // Ok: success = true, no error" % pad)
        L.append("%s    {" % pad)
        L.append("%s        var c = %s;" % (pad, self.s.new(child.name)))
        L.append("%s        %s" % (pad, self.s.set("c", succ, "true")))
        L.append("%s        %s" % (pad, self.s.set("m", f, "c")))
        L.append("%s        break;" % pad)
        L.append("%s    }" % pad)
        L.append("%s    case 2:   // Invalid: no child at all" % pad)
        L.append("%s        break;" % pad)
        L.append("%s    default:  // Error: success omitted, error set" % pad)
        L.append("%s    {" % pad)
        L.append("%s        var c = %s;" % (pad, self.s.new(child.name)))
        L.append("%s        %s" % (pad, self.s.set("c", err, self.text(err, path + ".error", "idx"))))
        L.append("%s        %s" % (pad, self.s.set("m", f, "c")))
        L.append("%s        break;" % pad)
        L.append("%s    }" % pad)
        L.append("%s}" % pad)

    def emit_oneof(self, L, msg, parent_path, mode):
        """A oneof carries exactly one member, cycled by element index.

        `payloads.py` does NOT apply the absent-path modes to the oneof: it
        calls `oneof_member` unconditionally. Reproduced rather than corrected,
        because the manifest is the oracle and it was built that way -- the
        payload set's only oneof is on M3, which no mode touches.
        """
        pad = "        "
        for oname, members in msg.oneofs.items():
            L.append("%sswitch (idx %% %d)" % (pad, len(members)))
            L.append("%s{" % pad)
            for i, f in enumerate(members):
                path = "%s.%s" % (msg.name, f.name)
                last = i == len(members) - 1
                L.append("%s    %s" % (pad, "default:" if last else "case %d:" % i))
                if f.kind == "string":
                    e = 'Values.Word("%s", idx)' % path
                elif f.kind == "bytes":
                    e = self.s.bytes_('Values.Blob("%s", idx)' % path)
                elif f.kind in ("int64", "int32"):
                    e = self.scalar(f.kind, path, "idx")
                elif f.kind == "bool":
                    e = self.scalar("bool", path, "idx")
                elif f.kind == "message" and f.of == "Empty":
                    e = self.s.new("Empty")
                elif f.kind == "message" and f.of == "Timestamp":
                    e = "NewStamp(idx)"
                elif f.kind == "message" and f.of == "Duration":
                    e = "NewDur(idx)"
                else:
                    raise KeyError("no case for oneof member %r" % f)
                L.append("%s        %s" % (pad, self.s.set_oneof("m", msg, f, e)))
                L.append("%s        break;" % pad)
            L.append("%s}" % pad)


def emit(ir, sink, roots):
    e = Emitter(ir, sink)
    plan = []
    for pid, spec in ir.payloads.items():
        root = ir.msg(spec["root"])
        mode = spec.get("mode")
        if "bulk" in spec:
            f = desc(root)[0]
            fn = e.fn_for(ir.msg(f.of), f.of, None, "Bulk")
            plan.append((pid, root, f, fn, spec, "bulk"))
        elif spec.get("interleaved"):
            left, right = desc(root)[0], desc(root)[1]
            fl = e.fn_for(ir.msg(left.of), left.of, None, "Element")
            fr = e.fn_for(ir.msg(right.of), right.of, None, "Element")
            plan.append((pid, root, (left, right), (fl, fr), spec, "interleaved"))
        else:
            f = next(x for x in desc(root) if x.name == spec["field"])
            fn = e.fn_for(ir.msg(f.of), f.of, mode, "Element")
            plan.append((pid, root, f, fn, spec, "list"))

    o = Head("Payload construction over the %s object model." % sink.name, "cs_build")
    o += "using System;"
    o += "using System.Collections.Generic;"
    if sink.name == "Gp":
        o += "using pb = Google.Protobuf;"
        o += "using Armonik.Ffi.Facade;"
        o += ""
        o += "namespace Armonik.Ffi.Harness;"
    else:
        o += ""
        o += "namespace Armonik.Ffi.Facade;"
    o += ""
    o += "public static class Build%s" % sink.name
    o += "{"
    o += "    private static %s NewStamp(long idx)" % sink.typ("Timestamp")
    o += "    {"
    o += "        var t = %s;" % sink.new("Timestamp")
    o += "        t.Seconds = Values.StampSeconds(idx); t.Nanos = Values.StampNanos(idx);"
    o += "        return t;"
    o += "    }"
    o += ""
    o += "    private static %s NewDur(long idx)" % sink.typ("Duration")
    o += "    {"
    o += "        var d = %s;" % sink.new("Duration")
    o += "        d.Seconds = Values.DurSeconds(idx); d.Nanos = Values.DurNanos(idx);"
    o += "        return d;"
    o += "    }"
    o += ""
    for ln in e.body:
        o += ln

    # ---- the roots ---------------------------------------------------
    for pid, root, f, fn, spec, kind in plan:
        m = "P" + pid[1:].replace(".", "_")
        o += "    /// %s: %s" % (pid, describe(spec, kind))
        o += "    public static %s %s()" % (sink.typ(root.name), m)
        o += "    {"
        o += "        var r = %s;" % sink.new(root.name)
        if kind == "bulk":
            o += "        r.%s = %s(0, 3, %d);" % (N.field(f.name), fn, spec["bulk"])
        elif kind == "interleaved":
            (left, right), (fl, fr) = f, fn
            o += "        // The wire INTERLEAVES the two fields; an object model cannot,"
            o += "        // so this graph is the re-encode permutation and not P7.1's bytes."
            o += "        for (int j = 0; j < %d; j++)" % spec["count"]
            o += "        {"
            o += "            r.%s.Add(%s(j, 3));" % (N.field(left.name), fl)
            o += "            r.%s.Add(%s(j, 3));" % (N.field(right.name), fr)
            o += "        }"
        else:
            reps = spec.get("repeats", 3)
            if isinstance(reps, list):
                o += "        int[] reps = { %s };" % ", ".join(str(x) for x in reps)
                o += "        for (int j = 0; j < %d; j++) r.%s.Add(%s(j, reps[j %% %d]));" % (
                    spec["count"], N.field(f.name), fn, len(reps))
            else:
                o += "        for (int j = 0; j < %d; j++) r.%s.Add(%s(j, %d));" % (
                    spec["count"], N.field(f.name), fn, reps)
            for x in desc(root):
                if x.name == "page":
                    o += "        r.%s = 1;" % N.field(x.name)
                if x.name == "total":
                    o += "        r.%s = %d;" % (N.field(x.name), spec["count"])
        o += "        return r;"
        o += "    }"
        o += ""
    o += "}"
    return str(o)


def describe(spec, kind):
    if kind == "bulk":
        return "one %d-byte bulk body" % spec["bulk"]
    if kind == "interleaved":
        return "%d + %d interleaved elements" % (spec["count"], spec["count"])
    return "%d elements%s" % (spec["count"],
                              ", mode " + spec["mode"] if spec.get("mode") else "")
