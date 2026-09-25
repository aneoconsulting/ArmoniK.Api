"""CAMPAIGN.md section 4.1: the codec suite, one process, pinned to AK_CPU_CLIENT, no server.

  python3.12 camp_codec.py --family shapes|unknown --launch N --rounds R --out FILE
                           [--target-ms T] [--allow-dirty] [--smoke]

Families (a family is one process: the shapes core and the corpus-schema core are both
`libak_core.so` and cannot share one):
  shapes    the 16 payloads of design/SHAPES.md, ASCII; P2.4 also in the Latin-1 and wide
            content sets (SHAPES.md's P10 row: content sets are measured on P2.4)
  unknown   the corpus's unknown-field rows, one per position class: U-root-all,
            U-nested-all, U-deep-all, U-element-all, U-leaf-all, U-oneof-all,
            U-chunkelem-all (every wire type at every position; the disputed U-map-entry
            is left out). CAMPAIGN.md 7 points at "the rows named in section 4.3", which
            names none; this is the selection, stated

Arms (requirement 8) and unknown-field modes (requirement 10):
  incumbent-prod   grpcio's generated marshaller path: Message.SerializeToString on encode
                   (a message BUILT through protobuf's setters) and Message.FromString on
                   decode. Unknown fields: upb's default (retained), stated
  incumbent-best   the library's other entry points, labelled: SerializePartialToString,
                   and ParseFromString into one reused message
  core-ffi         the generated shim over the core, C extension facade, push decode.
                   drop: timed. retain: PENDING the decision 11 port (the shim passes no
                   `ak_unk_opts`; the core is being given the mechanism in poc/codec now),
                   reported in the header, never timed as something it is not
  core-ffi-attr    labelled extra: the same shim over the plain facade (GetAttr/SetAttr)
  host-gen         the pure-Python codec generated from the same plan: drop AND retain
Directions (requirement 9): encode; decode (the bare call); decode+read (decode, then read
every field through the same plan for every arm -- upb's FromString is lazy, so only this
row is like for like).

Samples (requirements 21-25, 28): per round, per (payload, content, direction), the arms in
an order rotated by one each round; one sample = `iters` iterations timed with
CLOCK_THREAD_CPUTIME_ID (and wall beside it). `iters` is calibrated once per arm to the
target, before round 1, and then one full sample's worth of iterations is run per arm as
the warm-up (the same rule for every arm). The allocator is put in the long-lived state
first (M_TOP_PAD, allocator.py, J26). The collector is ON; `gc.collect()` runs before every
sample so each arm starts from the same collector state.

Requirement 11: every iteration serialises the SAME object graph again. Neither upb-python
nor the facades memoise a serialised size or form per instance, so nothing is amortised;
stated rather than worked around.
"""
import gc
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import camp_lib as L  # noqa: E402

AFFINITY = L.pin("AK_CPU_CLIENT")   # before any thread or allocation of note

import allocator  # noqa: E402
_WARM = allocator.warm_up()

ARGS = sys.argv[1:]


def opt(name, default=None, conv=str):
    return conv(ARGS[ARGS.index(name) + 1]) if name in ARGS else default


FAMILY = opt("--family", "shapes")
UNKNOWN_ROWS = ["U-root-all", "U-nested-all", "U-deep-all", "U-element-all", "U-leaf-all",
                "U-oneof-all", "U-chunkelem-all"]
CONTENT_PAYLOADS = ["P2.4"]
TARGET_MS = opt("--target-ms", 50.0, float)


# ------------------------------------------------------------------ content sets

def recode(s, cs):
    """The Rust slice's content-set rule (poc/rust/crates/shapes-values, `recode`): the
    character count kept, only the width changes. Latin-1: U+00A0..U+00FF (2 bytes);
    wide: U+4E00.. (3 bytes). Applied to every string value of the facade graph, map keys
    and values included."""
    if cs == "ascii" or not s:
        return s
    if cs == "latin1":
        return "".join(chr(0xA0 + ((ord(c) - 0x20) & 0xFFFFFFFF) % 0x60) for c in s)
    return "".join(chr(0x4E00 + ord(c) * 37 % 0x1000) for c in s)


def transform(fac, obj, msg, ctors, cs):
    """A copy of facade `obj` built with `ctors`, every string recoded into set `cs`."""
    import facts as F
    kw = {}
    for f, k, c in F.walk(fac, msg):
        v = getattr(obj, f["name"])
        if v is None:
            kw[f["name"]] = None
        elif c in ("repeated", "packed"):
            kw[f["name"]] = [transform(fac, x, f["of"], ctors, cs) if k == "message"
                             else recode(x, cs) if k == "string" else x for x in v]
        elif c == "map":
            kw[f["name"]] = {recode(a, cs): recode(b, cs) for a, b in v.items()}
        elif k == "message":
            kw[f["name"]] = transform(fac, v, f["of"], ctors, cs)
        elif k == "string":
            kw[f["name"]] = recode(v, cs)
        else:
            kw[f["name"]] = v
    for o in F.oneof_groups(fac, msg):
        kw["%s_case" % o] = getattr(obj, "%s_case" % o)
    return ctors[msg](**kw)


# ------------------------------------------------------------------ the cases

class Case:
    __slots__ = ("payload", "content", "dir", "arm", "mode", "fn", "iters")

    def __init__(self, payload, content, d, arm, mode, fn):
        self.payload, self.content, self.dir, self.arm, self.mode, self.fn = payload, content, d, arm, mode, fn
        self.iters = 1


def loop(f):
    def run(n):
        for _ in range(n):
            f()
    return run


def shapes_cases(log):
    import arms
    fac = arms.facade
    cases, gates = [], []
    R = None
    for pid in arms.PAYLOADS:
        root = arms.ROOT_OF[pid]
        R = arms._pb_root(pid)
        plan = arms._PLANS[root]
        for cs in ["ascii"] + (["latin1", "wide"] if pid in CONTENT_PAYLOADS else []):
            fp = arms.build_facade(pid, arms.CT_PLAIN)
            if cs != "ascii":
                fp = transform(fac, fp, root, arms.CT_PLAIN, cs)
            fc = transform(fac, fp, root, arms.CT_CEXT, "ascii")
            upb = R()
            arms._fill_pb(upb, fp, plan)
            if cs == "ascii":
                ref = arms.reference(pid)
            else:
                # No manifest covers these sets (SHAPES.md): the reference is the incumbent's
                # deterministic encoding, and every arm is checked against it.
                ref = upb.SerializeToString(deterministic=True)
            enc = {
                ("incumbent-prod", "incumbent-default"): upb.SerializeToString,
                ("incumbent-best", "incumbent-default"): upb.SerializePartialToString,
                ("core-ffi", "drop"): lambda _f=fc, _r=root: arms._ffi.encode("cext", _r, _f),
                ("core-ffi-attr", "drop"): lambda _f=fp, _r=root: arms._ffi.encode("attr", _r, _f),
                ("host-gen", "drop"): lambda _f=fp, _r=root: getattr(arms.pycodec, "encode_root_" + _r)(_f),
                ("host-gen", "retain"): lambda _f=fp, _r=root: getattr(arms.pycodec_retain, "encode_root_" + _r)(_f),
            }
            reuse = R()

            def best_dec(_b=ref, _m=reuse):
                _m.ParseFromString(_b)
                return _m
            dec = {
                ("incumbent-prod", "incumbent-default"): lambda _b=ref, _R=R: _R.FromString(_b),
                ("incumbent-best", "incumbent-default"): best_dec,
                ("core-ffi", "drop"): lambda _b=ref, _r=root: arms._ffi.decode("cext", _r, _b, arms.TY_CEXT),
                ("core-ffi-attr", "drop"): lambda _b=ref, _r=root: arms._ffi.decode("attr", _r, _b, arms.TY_PLAIN),
                ("host-gen", "drop"): lambda _b=ref, _r=root: getattr(arms.pycodec, "decode_root_" + _r)(_b, arms.CT_PLAIN),
                ("host-gen", "retain"): lambda _b=ref, _r=root: getattr(arms.pycodec_retain, "decode_root_" + _r)(_b, arms.CT_PLAIN),
            }
            # Correctness before timing, per case (requirement 26, in process): every encode
            # equals the reference (the incumbent may write another legal form: map order),
            # every decode re-encodes to it.
            for (arm, mode), f in enc.items():
                b = f()
                if b != ref and not (arm.startswith("incumbent") and R.FromString(b) == R.FromString(ref)) \
                        and not (pid in arms.DECODE_ONLY and R.FromString(b) == R.FromString(ref)):
                    gates.append("%s %s encode %s/%s: bytes differ from the reference" % (pid, cs, arm, mode))
            for (arm, mode), f in dec.items():
                o = f()
                if arm.startswith("incumbent"):
                    back = o.SerializeToString(deterministic=True)
                    okb = back == ref or R.FromString(back) == R.FromString(ref)
                elif arm == "core-ffi":
                    back = arms._ffi.encode("cext", root, o)
                    okb = back == ref or (pid in arms.DECODE_ONLY and R.FromString(back) == R.FromString(ref))
                else:
                    back = arms._ffi.encode("attr", root, o)
                    okb = back == ref or (pid in arms.DECODE_ONLY and R.FromString(back) == R.FromString(ref))
                if not okb:
                    gates.append("%s %s decode %s/%s: does not re-encode to the reference" % (pid, cs, arm, mode))
            for (arm, mode), f in enc.items():
                cases.append(Case(pid, cs, "encode", arm, mode, loop(f)))
            for (arm, mode), f in dec.items():
                cases.append(Case(pid, cs, "decode", arm, mode, loop(f)))
                if arm.startswith("incumbent"):
                    rd = (lambda _f=f, _p=plan: arms._read_pb(_f(), _p, True))
                else:
                    rd = (lambda _f=f, _p=plan: arms._read(_f(), _p))
                cases.append(Case(pid, cs, "decode+read", arm, mode, loop(rd)))
    log.note("core-ffi retain: PENDING decision 11 port -- not timed (requirement 10)")
    log.note("payload set: %s; content sets latin1/wide on %s" % (", ".join(arms.PAYLOADS), ", ".join(CONTENT_PAYLOADS)))
    return cases, gates


def unknown_cases(log):
    """The unknown-field rows, through the corpus-schema core and the corpus plan's codec."""
    import json
    tag = "py%d.%d" % sys.version_info[:2]
    gen = os.path.join(HERE, "gen", "out", "corpus")
    for p in (gen, os.path.join(HERE, "build", tag), os.path.join(HERE, "build", tag, "pb2corpus")):
        sys.path.insert(0, p)
    import facade as fac          # noqa: E402  gen/out/corpus/facade.py
    import pycodec as pyd         # noqa: E402
    import pycodec_retain as pyr  # noqa: E402
    import _akffi_corpus as ffi   # noqa: E402
    import corpus_pb2 as pb       # noqa: E402  from corpus/generated/corpus.proto (reader)
    import arms_plan
    names = [n[1:] for n in ffi.types()]
    CP = {n: getattr(fac, "Plain" + n) for n in fac.MESSAGES}
    CC = {n: getattr(ffi, "C" + n) for n in names}
    TC = tuple(CC[n] for n in names)
    man = json.load(open(os.path.join(L.FFI, "corpus", "generated", "manifest.json")))["vectors"]
    cases, gates = [], []
    for vid in UNKNOWN_ROWS:
        r = man[vid]
        root = r["root"]
        buf = open(os.path.normpath(os.path.join(L.FFI, "corpus", "generated", r["file"])), "rb").read()
        R = getattr(pb, root)
        plan = arms_plan.plan(fac, root)
        m = R.FromString(buf)
        op = pyd.__dict__["decode_root_" + root](buf, CP)
        opr = pyr.__dict__["decode_root_" + root](buf, CP)
        oc = ffi.decode("cext", root, buf, TC)
        accepted = {a["sha256"] for a in r.get("accepted_encodings", [])}
        import hashlib
        enc = {
            ("incumbent-prod", "incumbent-default"): m.SerializeToString,
            ("core-ffi", "drop"): lambda _o=oc, _r=root: ffi.encode("cext", _r, _o),
            ("host-gen", "drop"): lambda _o=op, _r=root: pyd.__dict__["encode_root_" + _r](_o),
            ("host-gen", "retain"): lambda _o=opr, _r=root: pyr.__dict__["encode_root_" + _r](_o),
        }
        dec = {
            ("incumbent-prod", "incumbent-default"): lambda _b=buf, _R=R: _R.FromString(_b),
            ("core-ffi", "drop"): lambda _b=buf, _r=root: ffi.decode("cext", _r, _b, TC),
            ("host-gen", "drop"): lambda _b=buf, _r=root: pyd.__dict__["decode_root_" + _r](_b, CP),
            ("host-gen", "retain"): lambda _b=buf, _r=root: pyr.__dict__["decode_root_" + _r](_b, CP),
        }
        for (arm, mode), f in enc.items():
            b = f()
            h = hashlib.sha256(b).hexdigest()
            # The incumbent may write another legal form (upb's map order, its placement of
            # the retained unknown runs): it is checked by parsing to the same message.
            if arm.startswith("incumbent") and R.FromString(b) == m:
                continue
            if accepted and h not in accepted:
                gates.append("%s encode %s/%s: not an accepted encoding" % (vid, arm, mode))
        for (arm, mode), f in enc.items():
            cases.append(Case(vid, "ascii", "encode", arm, mode, loop(f)))
        for (arm, mode), f in dec.items():
            cases.append(Case(vid, "ascii", "decode", arm, mode, loop(f)))
            import arms_plan as AP
            rd = ((lambda _f=f, _p=plan: AP.read_pb(_f(), _p)) if arm.startswith("incumbent")
                  else (lambda _f=f, _p=plan: AP.read(_f(), _p)))
            cases.append(Case(vid, "ascii", "decode+read", arm, mode, loop(rd)))
    log.note("core-ffi retain: PENDING decision 11 port -- not timed (requirement 10)")
    log.note("unknown-field rows: %s" % ", ".join(UNKNOWN_ROWS))
    return cases, gates


def calibrate(c, target_ns):
    n = 1
    while True:
        t = L.thread_cpu_ns()
        c.fn(n)
        dt = L.thread_cpu_ns() - t
        if dt >= target_ns or n >= 1 << 24:
            c.iters = n
            return
        n = max(n + 1, int(n * min(16.0, max(2.0, target_ns / max(dt, 1)))))


def main():
    launch = opt("--launch", 1, int)
    rounds = opt("--rounds", 5, int)
    log = L.Log(opt("--out"), "codec", allow_dirty="--allow-dirty" in ARGS, smoke="--smoke" in ARGS)
    log.header(family=FAMILY, launch=launch, rounds=rounds, target_ms_per_sample=TARGET_MS,
               affinity=AFFINITY, allocator="mallopt(M_TOP_PAD, 8 MiB) %s" % ("applied" if _WARM else "NOT AVAILABLE"),
               gc="ON; gc.collect() before every sample",
               warmup="per arm, before round 1: calibration to the target, then one sample's iterations",
               clock="CLOCK_THREAD_CPUTIME_ID (cpu_ns), perf_counter_ns (wall_ns), totals over iters",
               incumbent_path="Message.SerializeToString / Message.FromString (grpcio's generated marshaller)",
               core_ffi_unknown="drop timed; retain pending the decision 11 port")
    cases, gates = (shapes_cases if FAMILY == "shapes" else unknown_cases)(log)
    if gates:
        log.close(False, "correctness gate failed before timing: " + "; ".join(gates[:5]))
        print("\n".join(gates))
        return 1
    target = int(TARGET_MS * 1e6)
    for c in cases:
        calibrate(c, target)
        c.fn(c.iters)                      # the warm-up: one sample's worth, every arm alike
    groups = {}
    for c in cases:
        groups.setdefault((c.payload, c.content, c.dir), []).append(c)
    for r in range(rounds):
        for key, cs in groups.items():
            for c in L.rotated(cs, r):
                gc.collect()
                t0, w0 = L.thread_cpu_ns(), L.wall_ns()
                c.fn(c.iters)
                t1, w1 = L.thread_cpu_ns(), L.wall_ns()
                log.sample(arm=c.arm, payload=c.payload, content=c.content, dir=c.dir,
                           unknown_mode=c.mode, launch=launch, round=r + 1,
                           cpu_ns=t1 - t0, wall_ns=w1 - w0, iters=c.iters)
    log.close(True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
