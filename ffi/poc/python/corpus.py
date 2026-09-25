"""W8 / FIX-PLAN WP5 item 6.1: the WHOLE conformance corpus through every python arm.

`ffi/corpus/CONTRACT.md` is the obligation. Five arms, each row in a worker process under a
timeout (a hang or a crash is a row result, not the end of the run):

| arm | what | unknown fields |
|---|---|---|
| `ffi-cext`   | `_akffi_corpus`: the corpus-schema core (`corpus,init-guard`) behind the generated shim, C extension facade | dropped |
| `ffi-attr`   | the same shim, the plain facade through `PyObject_GetAttr/SetAttr` | dropped |
| `ffi-chunk256` | the same shim built with 256-byte element chunks and 3-value packed runs (C ext facade), so every run crosses in many chunks | dropped |
| `ffi-retain` | the same shim, C ext facade, decision 11 armed at every position (`ak_dec_<Root>_opts`, the host's grow) and the `ak_uencode_*` family on the way back | retained |
| `py-drop`    | the generated pure-Python codec (`gen/out/corpus/pycodec.py`), plain facade | dropped |
| `py-retain`  | the same backend in retain mode (`pycodec_retain.py`) | retained |

Every generated file is rendered by `poc/codec/gen` from the corpus READER schema's plan
(CONTRACT.md rule 0: never the superset). The shim covers every root the C ABI can carry;
`Nest` is recursive and is REFUSED by the C ABI by name (plan.check_expressible), so its
rows are "not in the C ABI" on the two ffi arms and run on the pure-Python arms.
Retention through the C ABI is not rendered in this shim, so there is no `ffi-retain` arm.

Obligations: C1 parse, C2 project, C3 re-encode to an accepted form (and say which), C4
refuse (and record the code), C5 produce (rows naming `python`: build the message from the
projection with the arm's own facade constructors and encode it). Disputed rows are
excluded from pass/fail and the reading this slice produced is reported. Between-arm byte
identity (CONTRACT 5.5) is checked on every accepted row for the three drop arms.

Controls, each of which MUST fail (`--controls`):
  proj    a key planted in every projection                -> C2 must fail
  reenc   a byte appended to every re-encoding             -> C3 must fail
  accept  every refusal turned into an acceptance          -> C4 must fail
  noinit  the shim built with ak_init skipped (AK_SKIP_INIT) against the guarded core
                                                           -> every ffi row must fail
  python3.12 corpus.py [--arms a,b] [--only prefix,...] [--timeout s] [--controls]
                       [--dump FILE] [--compare FILE]   (re-encoding sha256s, per arm and row)

THE NO-UNKNOWN VARIANT (WP5 step 10), `AK_NOUNK=1` in the environment: the ffi arms import
`_akffi_corpus_nounk` / `_akffi_corpus_chunk_nounk` (the shim rendered from the corpus plan
relowered with unknown="drop", over ak-core `--no-default-features --features
corpus,init-guard`); the arms are ffi-cext, ffi-attr, ffi-chunk256 and py-drop. Added checks:
every unknown-class row writes the DROPPED form on every ffi arm, and `--compare` against the
full build's dump at the same level is the byte identity against drop. `--d11` in the
variant: retain is refused, no root has a position, wrong root is refused (-8, no reset
exists), and the per-thread contexts control in drop mode.
"""
import hashlib
import json
import os
import select
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
FFI = os.path.dirname(os.path.dirname(HERE))
CORPUS = os.path.join(FFI, "corpus", "generated")
TAG = "py%d.%d" % sys.version_info[:2]
GEN = os.path.join(HERE, "gen", "out", "corpus")
ARMS = ["ffi-cext", "ffi-attr", "ffi-chunk256", "ffi-retain", "py-drop", "py-retain"]
DROP_ARMS = ["ffi-cext", "ffi-attr", "ffi-chunk256", "py-drop"]
RETAIN_ARMS = ["ffi-retain", "py-retain"]
NOUNK = os.environ.get("AK_NOUNK") == "1"
SFX = "_nounk" if NOUNK else ""
if NOUNK:
    GEN = os.path.join(HERE, "gen", "out", "corpus-nounk")   # the facade without `_unknown`
    ARMS = list(DROP_ARMS)
    RETAIN_ARMS = []


def rpath(rel):
    return os.path.normpath(os.path.join(CORPUS, rel))


# ====================================================================== worker side

class Arm:
    """One arm inside a worker: decode(buf, root), encode(obj, root), ctors, roots."""

    def __init__(self, name):
        sys.path.insert(0, GEN)
        sys.path.insert(0, HERE)
        import facade           # noqa: E402  gen/out/corpus/facade.py
        self.fac = facade
        self.name = name
        self.plant = os.environ.get("AK_CORPUS_PLANT", "")
        plain = {n: getattr(facade, "Plain" + n) for n in facade.MESSAGES}
        if name.startswith("ffi"):
            self.chunk_bytes = 32768
            if self.plant == "noinit":
                sys.path.insert(0, os.path.join(HERE, "build", TAG, "ctl"))
                mod = __import__("_akffi_corpus_noinit" + SFX)
            elif name == "ffi-chunk256":
                sys.path.insert(0, os.path.join(HERE, "build", TAG))
                mod = __import__("_akffi_corpus_chunk" + SFX)
                self.chunk_bytes = 256
            else:
                sys.path.insert(0, os.path.join(HERE, "build", TAG))
                mod = __import__("_akffi_corpus" + SFX)
            if bool(getattr(mod, "nounk", lambda: False)()) != NOUNK:
                raise SystemExit("%s is not the %s variant" % (mod.__name__, "no-unknown" if NOUNK else "full"))
            self.mod = mod
            names = [n[1:] for n in mod.types()]
            self.roots = set(mod.roots())
            self.layout = dict(mod.layout_host())
            b = "attr" if name == "ffi-attr" else "cext"
            retain = name == "ffi-retain"
            if b == "cext":
                self.ctors = {n: getattr(mod, "C" + n) for n in names}
            else:
                self.ctors = plain
            ty = tuple(self.ctors[n] for n in names)
            self.decode = lambda buf, root: mod.decode(b, root, buf, ty, None, retain)
            self.encode = lambda obj, root: mod.encode(b, root, obj, None, retain)
        else:
            codec = __import__("pycodec_retain" if name == "py-retain" else "pycodec")
            self.codec = codec
            self.roots = set(facade.MESSAGES)
            self.ctors = plain
            self.layout = {}
            self.decode = lambda buf, root: getattr(codec, "decode_root_" + root)(buf, plain)
            self.encode = lambda obj, root: getattr(codec, "encode_root_" + root)(obj)

    # ---- CONTRACT.md section 3, over the facade's plan-rendered facts
    def project(self, obj, msg):
        import facts as F
        out = {}
        for f, k, c in F.walk(self.fac, msg):
            nm = f["name"]
            if c == "oneof":
                continue
            v = getattr(obj, nm)
            if c == "optional":
                if v is not None:
                    out[nm] = _pval(k, v)
            elif c in ("repeated", "packed"):
                if v:
                    out[nm] = [self.project(x, f["of"]) if k == "message" else _pval(k, x) for x in v]
            elif c == "map":
                if v:
                    ek, evk = f["entry"]
                    out[nm] = {_pkey(ek, a): _pval(evk, b) for a, b in v.items()}
            elif k == "message":
                if v is not None:
                    out[nm] = self.project(v, f["of"])
            elif _nonzero(k, v):
                out[nm] = _pval(k, v)
        for oname, members in F.oneof_groups(self.fac, msg).items():
            tag = getattr(obj, "%s_case" % oname)
            g = next((x for x in members if x["tag"] == tag), None)
            if g is None:
                continue
            v = getattr(obj, g["name"])
            out[g["name"]] = self.project(v, g["of"]) if g["kind"] == "message" else _pval(g["kind"], v)
        return out

    def unproject(self, d, msg):
        """C5: the message a projection describes, built with THIS arm's constructors."""
        import facts as F
        kw = {}
        for f, k, c in F.walk(self.fac, msg):
            nm = f["name"]
            if nm not in d:
                continue
            v = d[nm]
            if c == "oneof":
                kw[nm] = self.unproject(v, f["of"]) if k == "message" else _uval(k, v)
                kw["%s_case" % f["oneof"]] = f["tag"]
            elif c in ("repeated", "packed"):
                kw[nm] = [self.unproject(x, f["of"]) if k == "message" else _uval(k, x) for x in v]
            elif c == "map":
                ek, evk = f["entry"]
                kw[nm] = {_uval(ek, a): _uval(evk, b) for a, b in v.items()}
            elif k == "message":
                kw[nm] = self.unproject(v, f["of"])
            else:
                kw[nm] = _uval(k, v)
        return self.ctors[msg](**kw)


def _nonzero(k, v):
    if k == "double":
        return v != 0.0 or str(v).startswith("-")
    return bool(v)


def _pval(k, v):
    if k == "string":
        return v
    if k == "bytes":
        return bytes(v).hex()
    if k == "bool":
        return bool(v)
    if k == "double":
        return "%.17g" % v
    return str(int(v))


def _pkey(k, v):
    return v if k == "string" else str(v)


def _uval(k, v):
    if k == "string":
        return v
    if k == "bytes":
        return bytes.fromhex(v)
    if k == "bool":
        return bool(v)
    if k == "double":
        return float(v)
    return int(v)


def strip_unknown(d):
    if isinstance(d, dict):
        return {k: strip_unknown(v) for k, v in d.items() if k != "_unknown"}
    if isinstance(d, list):
        return [strip_unknown(x) for x in d]
    return d


def err_code(e):
    c = getattr(e, "code", None)
    if isinstance(c, int):
        return c, "%s(%d)" % (type(e).__name__, c)
    s = str(e)
    if "returned " in s:
        try:
            return int(s.rsplit("returned ", 1)[1].split()[0]), s
        except ValueError:
            pass
    return None, "%s: %s" % (type(e).__name__, s[:80])


def fields_of(b):
    """The top-level (tag, wire, raw field bytes) of an encoding, for the permutation rule."""
    out, i, n = [], 0, len(b)

    def vr(i):
        x = s = 0
        while True:
            c = b[i]
            i += 1
            x |= (c & 0x7F) << s
            if c < 128:
                return x, i
            s += 7
    while i < n:
        s0 = i
        k, i = vr(i)
        w = k & 7
        if w == 0:
            _, i = vr(i)
        elif w == 1:
            i += 8
        elif w == 5:
            i += 4
        elif w == 2:
            ln, i = vr(i)
            i += ln
        else:
            return None
        out.append(bytes(b[s0:i]))
    return sorted(out)


def check_encoding(r, back, buf):
    """Which accepted form `back` is, or None."""
    h = hashlib.sha256(back).hexdigest()
    encs = r.get("accepted_encodings") or [{"sha256": r["sha256"], "forms": ["as committed"]}]
    for a in encs:
        if a["sha256"] == h:
            return " / ".join(a.get("forms", ["?"]))
    if r.get("permutation_accepted"):
        fb = fields_of(back)
        if fb is not None and fb == fields_of(buf):
            return "a permutation of the committed form (permutation_accepted)"
    return None


def run_row(arm, vid, r):
    plant = arm.plant
    res = {"id": vid, "fails": [], "c": {}, "form": None, "err": None, "enc": None,
           "disputed": None, "chunks": None}
    root = r["root"]
    if root not in arm.roots:
        res["notabi"] = True
        return res
    buf = open(rpath(r["file"]), "rb").read()
    disputed = r.get("verdict") == "disputed"
    try:
        obj = arm.decode(buf, root)
        err = None
    except Exception as e:  # noqa: BLE001
        obj, err = None, e
    if plant == "accept" and err is not None:
        obj, err = None, None
    if r["expect"] == "reject":
        if disputed:
            res["disputed"] = ("refused (%s)" % err_code(err)[1]) if err is not None else "accepted"
            return res
        if err is None:
            res["fails"].append("C4: accepted a reject vector")
        else:
            res["c"]["C4"] = 1
            res["err"] = err_code(err)[1]
        return res
    if err is not None:
        if disputed:
            res["disputed"] = "refused (%s)" % err_code(err)[1]
        else:
            res["fails"].append("C1: refused with %s" % err_code(err)[1])
        return res
    res["c"]["C1"] = 1
    got = arm.project(obj, root)
    if disputed:
        rd = []
        for reading in (r.get("dispute") or {}).get("readings", []):
            want = strip_unknown(json.load(open(rpath(reading["projection"]))))
            if want == got:
                rd.append(", ".join(reading["read_by"]))
        res["disputed"] = "accepted; projection matches the reading of %s" % (" + ".join(rd) or "NONE")
        return res
    if r.get("projection"):
        want = strip_unknown(json.load(open(rpath(r["projection"]))))
        if plant == "proj":
            want = dict(want, _planted=True)
        if got == want:
            res["c"]["C2"] = 1
        else:
            res["fails"].append("C2: projection differs: %s" % _pdiff(got, want))
    try:
        back = arm.encode(obj, root)
    except Exception as e:  # noqa: BLE001
        res["fails"].append("C3: re-encode raised %s" % err_code(e)[1])
        return res
    if plant == "reenc":
        back = back + b"\x00"
    res["enc"] = hashlib.sha256(back).hexdigest()
    form = check_encoding(r, back, buf)
    if form is None:
        res["fails"].append("C3: wrote a form the manifest does not accept (%d B, sha %s)"
                            % (len(back), res["enc"][:12]))
    else:
        res["c"]["C3"] = 1
        res["form"] = form
    if "python" in r.get("produce", []):
        if r.get("projection"):
            try:
                built = arm.unproject(json.load(open(rpath(r["projection"]))), root)
                pb = arm.encode(built, root)
                if plant == "reenc":
                    pb = pb + b"\x00"
                pf = check_encoding(r, pb, buf)
                if pf is None:
                    res["fails"].append("C5: produced a form the manifest does not accept (%d B)" % len(pb))
                else:
                    res["c"]["C5"] = 1
            except Exception as e:  # noqa: BLE001
                res["fails"].append("C5: raised %s" % err_code(e)[1])
        else:
            res["c"]["C5-via-decode"] = 1
    if r["class"] == "chunking" and arm.layout:
        res["chunks"] = chunk_report(arm, obj, root)
    return res


def chunk_report(arm, obj, root):
    """CONTRACT 4 `chunking`: how many element chunks this host's encode made, from the
    shim's own group sizes (ABI v1: 32 KB of host-side element groups per chunk)."""
    import facts as F
    out = []
    for f, k, c in F.walk(arm.fac, root):
        if c == "repeated" and k == "message":
            n = len(getattr(obj, f["name"]))
            size = arm.layout.get("sizeof ak_efix_%s" % f["of"])
            if not size:
                continue
            per = max(1, arm.chunk_bytes // size)
            out.append("%s: %d x %s, group %d B, %d per chunk -> %d chunk(s)"
                       % (f["name"], n, f["of"], size, per, (n + per - 1) // per if n else 0))
    return "; ".join(out)


def _pdiff(got, want):
    gk, wk = set(got), set(want)
    if gk != wk:
        return "keys +%s -%s" % (sorted(gk - wk)[:3], sorted(wk - gk)[:3])
    for k in sorted(gk):
        if got[k] != want[k]:
            return "%s: %r vs %r" % (k, str(got[k])[:60], str(want[k])[:60])
    return "equal?"


def worker(armname):
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))["vectors"]
    try:
        arm = Arm(armname)
    except Exception as e:  # noqa: BLE001
        sys.stdout.write(json.dumps({"init_error": "%s: %s" % (type(e).__name__, e)}) + "\n")
        sys.stdout.flush()
        return 0
    sys.stdout.write(json.dumps({"ready": True}) + "\n")
    sys.stdout.flush()
    for line in sys.stdin:
        vid = line.strip()
        if not vid:
            continue
        try:
            res = run_row(arm, vid, man[vid])
        except Exception as e:  # noqa: BLE001
            res = {"id": vid, "fails": ["harness: %s: %s" % (type(e).__name__, e)], "c": {}}
        sys.stdout.write(json.dumps(res) + "\n")
        sys.stdout.flush()
    return 0


# ====================================================================== parent side

class Worker:
    def __init__(self, arm, env):
        self.arm, self.env = arm, env
        self.p = None
        self.start()

    def start(self):
        self.p = subprocess.Popen([sys.executable, os.path.abspath(__file__), "--worker", self.arm],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                  env=self.env, text=True, bufsize=1)
        first = self._read(60)
        self.init_error = None
        if first is None or "ready" not in first:
            # An arm that cannot even load (the noinit control may be refused at import)
            # fails every row, loudly, rather than aborting the run.
            self.init_error = (first or {}).get("init_error", "worker did not start: %r" % (first,))

    def _read(self, timeout):
        r, _, _ = select.select([self.p.stdout], [], [], timeout)
        if not r:
            return None
        line = self.p.stdout.readline()
        if not line:
            return {"eof": True}
        return json.loads(line)

    def row(self, vid, timeout):
        if self.init_error:
            return {"id": vid, "fails": ["arm did not load: %s" % self.init_error[:120]], "c": {}}
        self.p.stdin.write(vid + "\n")
        self.p.stdin.flush()
        t0 = time.time()
        res = self._read(timeout)
        if res is None:
            self.p.kill()
            self.p.wait()
            self.start()
            return {"id": vid, "fails": ["HANG: no answer within %.0f s" % timeout], "c": {}, "hung": True}
        if res.get("eof"):
            rc = self.p.wait()
            self.start()
            return {"id": vid, "fails": ["CRASH: worker died (exit %s) after %.1f s" % (rc, time.time() - t0)],
                    "c": {}, "crashed": True}
        return res

    def close(self):
        try:
            self.p.stdin.close()
            self.p.wait(timeout=10)
        except Exception:  # noqa: BLE001
            self.p.kill()


def run(arms, only, timeout, plant=None, quiet=False):
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))
    rows = man["vectors"]
    ids = sorted(rows)
    if only:
        ids = [i for i in ids if any(i.startswith(p) for p in only)]
    env = dict(os.environ)
    if plant:
        env["AK_CORPUS_PLANT"] = plant
    results = {}
    for arm in arms:
        w = Worker(arm, env)
        res = {}
        for vid in ids:
            res[vid] = w.row(vid, timeout)
        w.close()
        results[arm] = res
    return man, ids, results


def summarise(man, ids, results, out=sys.stdout):
    rows = man["vectors"]
    fails = 0
    hung = 0
    classes = sorted({rows[i]["class"] for i in ids})
    for arm, res in results.items():
        p = f = d = na = 0
        per = {c: [0, 0] for c in classes}
        forms = {}
        cnt = {"C1": 0, "C2": 0, "C3": 0, "C4": 0, "C5": 0, "C5-via-decode": 0}
        for vid in ids:
            r = res[vid]
            cls = rows[vid]["class"]
            if r.get("notabi"):
                na += 1
                continue
            if r.get("disputed"):
                d += 1
                continue
            for k in r.get("c", {}):
                cnt[k] = cnt.get(k, 0) + 1
            if r.get("hung") or r.get("crashed"):
                hung += 1
            if r["fails"]:
                f += 1
                per[cls][1] += 1
            else:
                p += 1
                per[cls][0] += 1
            if r.get("form"):
                forms[r["form"]] = forms.get(r["form"], 0) + 1
        print("\n## %s" % arm, file=out)
        print("   pass %d  fail %d  disputed (excluded) %d  not in the C ABI %d" % (p, f, d, na), file=out)
        print("   obligations met: %s" % ", ".join("%s=%d" % kv for kv in cnt.items()), file=out)
        for c in classes:
            print("     %-10s pass %4d  fail %3d" % (c, per[c][0], per[c][1]), file=out)
        print("   forms written (C3):", file=out)
        for k, v in sorted(forms.items(), key=lambda x: (-x[1], x[0])):
            print("     %5d  %s" % (v, k), file=out)
        for vid in ids:
            r = res[vid]
            if r.get("disputed"):
                print("   disputed: %s [%s]: %s" % (vid, arm, r["disputed"]), file=out)
        for vid in ids:
            r = res[vid]
            for x in r.get("fails", []):
                print("   FAIL %s [%s]: %s" % (vid, arm, x), file=out)
                fails += 1
        na_ids = [vid for vid in ids if res[vid].get("notabi")]
        if na_ids:
            print("   not in the C ABI (root refused by plan.check_expressible): %s" % ", ".join(na_ids), file=out)
        via = [vid for vid in ids if "C5-via-decode" in res[vid].get("c", {})]
        if via:
            print("   C5 rows with no projection to build from (checked by C3 re-encode only): %s"
                  % ", ".join(via), file=out)
        ch = [(vid, res[vid]["chunks"]) for vid in ids if res[vid].get("chunks")]
        for vid, c in ch:
            print("   chunking %s [%s]: %s" % (vid, arm, c), file=out)
    return fails, hung


def cross_arm(ids, results, out=sys.stdout, family=None):
    """CONTRACT 5.5: byte identity between the slice's own arms, per unknown-field mode."""
    arms = [a for a in (family or DROP_ARMS) if a in results]
    bad = 0
    n = 0
    for vid in ids:
        encs = {a: results[a][vid].get("enc") for a in arms
                if not results[a][vid].get("notabi") and results[a][vid].get("enc")}
        if len(encs) < 2:
            continue
        n += 1
        if len(set(encs.values())) != 1:
            bad += 1
            print("   DIFFER %s: %s" % (vid, ", ".join("%s=%s" % (a, h[:10]) for a, h in encs.items())), file=out)
    print("   %d rows re-encoded by at least two of these arms; %d differ" % (n, bad), file=out)
    return bad


def refusals(ids, man, results, out=sys.stdout):
    rows = man["vectors"]
    rej = [i for i in ids if rows[i]["expect"] == "reject"]
    arms = list(results)
    print("   %-40s %s" % ("vector", " | ".join(arms)), file=out)
    for vid in rej:
        cells = []
        for a in arms:
            r = results[a][vid]
            cells.append("n/a" if r.get("notabi") else (r.get("err") or r.get("disputed") or
                                                         ("; ".join(r.get("fails", [])) or "?")))
        print("   %-40s %s" % (vid, " | ".join(c[:28] for c in cells)), file=out)


def d11_controls(out=sys.stdout):
    """Decision 11's controls through the C ABI, in one process (the ffi-retain arm's shim).

    zeroed position: on every `unknown`-class row this shim can decode, for every position of
    its root, decode with that position's entry left all-zero; the value must be the
    all-armed value with exactly that position's bags cleared (known fields unchanged).
    wrong root (rule 6): a context bound to root a, reset and decode as root b: both refused
    with AK_ERR_INVALID_STATE (-8) and nothing delivered; a == b is the positive control
    (both succeed on an empty input and the trap apply runs).
    leak: every successful retain decode reclaims 0 undelivered buffers."""
    import facts as F
    arm = Arm("ffi-retain")
    mod, fac = arm.mod, arm.fac
    names = [n[1:] for n in mod.types()]
    ty = tuple(arm.ctors[n] for n in names)
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))["vectors"]

    def bags(obj, msg, pos=(), key=(), acc=None):
        acc = {} if acc is None else acc
        u = getattr(obj, "_unknown", b"") or b""
        if u:
            acc[key] = (".".join(pos), bytes(u))
        for f, k, c in F.walk(fac, msg):
            if k != "message":
                continue
            v = getattr(obj, f["name"])
            if c == "oneof":
                if v is not None and getattr(obj, "%s_case" % f["oneof"]) == f["tag"]:
                    bags(v, f["of"], pos + (f["oneof"],), key + (f["name"],), acc)
            elif c == "repeated":
                for i, x in enumerate(v):
                    bags(x, f["of"], pos + (f["name"],), key + ((f["name"], i),), acc)
            elif v is not None:
                bags(v, f["of"], pos + (f["name"],), key + (f["name"],), acc)
        return acc
    bad = 0
    rows = sorted(k for k, r in man.items() if r["class"] == "unknown" and r["expect"] == "accept"
                  and r.get("verdict") != "disputed" and r["root"] in arm.roots)
    checked = changed = 0
    leaks = []
    for vid in rows:
        r = man[vid]
        root = r["root"]
        buf = open(rpath(r["file"]), "rb").read()
        A = mod.decode("cext", root, buf, ty, None, True)
        if mod.last_reclaimed():
            leaks.append(vid)
        bA = bags(A, root)
        pA = strip_unknown(arm.project(A, root))
        for i, entry in enumerate(mod.unk_positions(root)):
            _n, _kind, path = entry.split("|")
            o = mod.decode("cext", root, buf, ty, None, True, 1 << i)
            want = {k: v for k, v in bA.items() if v[0] != path}
            got = bags(o, root)
            checked += 1
            if len(want) != len(bA):
                changed += 1
            if got != want or strip_unknown(arm.project(o, root)) != pA:
                bad += 1
                print("   ZEROED-POSITION MISMATCH %s position %d (%s): got %d bag(s), want %d"
                      % (vid, i, entry, len(got), len(want)), file=out)
    print("   zeroed position: %d (row, position) pairs over %d rows; %d where zeroing removes a bag; %d mismatch"
          % (checked, len(rows), changed, bad), file=out)
    print("   leak: %d successful retain decode(s) reclaimed an undelivered buffer%s"
          % (len(leaks), (": " + ", ".join(leaks)) if leaks else ""), file=out)
    bad += len(leaks)
    if changed == 0:
        print("   ZEROED-POSITION CONTROL IS BLIND: no zeroing removed any bag", file=out)
        bad += 1
    roots = sorted(arm.roots)
    wr_bad = pos_ok = 0
    for a in roots:
        for b in roots:
            rr, dr, delivered = mod.wrong_root(a, b)
            if a == b:
                if rr == 0 and dr == 0 and delivered:
                    pos_ok += 1
                else:
                    wr_bad += 1
                    print("   WRONG-ROOT positive control failed for %s: %r" % (a, (rr, dr, delivered)), file=out)
            elif not (rr == -8 and dr == -8 and not delivered):
                wr_bad += 1
                print("   WRONG ROOT ACCEPTED: ctx %s, root %s -> reset %d decode %d delivered %s"
                      % (a, b, rr, dr, delivered), file=out)
    print("   wrong root: %d ordered pairs refused with -8 and nothing delivered; positive control %d of %d roots; %d failure(s)"
          % (len(roots) * (len(roots) - 1) - (wr_bad if wr_bad else 0), pos_ok, len(roots), wr_bad), file=out)
    bad += wr_bad
    bad += tls_control(mod, ty, man, rows, out)
    ok, detail = reclaim_tls_control(mod, ty, man, rows, out)
    print("   per-thread AK_LAST_RECLAIMED: %s (%s)" % ("holds" if ok else "FAILS", detail), file=out)
    bad += 0 if ok else 1
    # Its must-fail twin: the same shim built with AK_THREAD_LOCAL empty (one process-wide slot).
    sys.path.insert(0, os.path.join(HERE, "build", TAG, "ctl"))
    try:
        gmod = __import__("_akffi_corpus_globalreclaim")
        gty = tuple(getattr(gmod, "C" + n) for n in names)
        gok, gdetail = reclaim_tls_control(gmod, gty, man, rows, out, label="process-wide slot")
        print("   must-fail twin: %s (%s)" % ("PASSED -- the check is blind" if gok else "failed as required", gdetail), file=out)
        bad += 1 if gok else 0
    except ImportError as e:
        print("   must-fail twin NOT BUILT: %s" % e, file=out)
        bad += 1
    print("D11 CONTROLS %s" % ("PASS" if not bad else "FAIL (%d)" % bad), file=out)
    return 1 if bad else 0


def reclaim_tls_control(mod, ty, man, rows, out, label="shim"):
    """AK_LAST_RECLAIMED is per thread. Thread A makes a retain decode that FAILS after
    growing buffers (an unknown-bearing row with a truncated field appended), reads its
    count n1 > 0, and waits; thread B then makes a successful decode (its own count 0) and
    signals; A reads again and must still see n1. With one process-wide slot A sees B's 0.
    Returns (verdict ok, detail)."""
    import threading
    vid = None
    for v in rows:
        r = man[v]
        b = open(rpath(r["file"]), "rb").read() + b"\x0a\x7f"   # field 1, wire type 2, 127 bytes promised, none given
        try:
            mod.decode("cext", r["root"], b, ty, None, True)
        except Exception:  # noqa: BLE001 -- the failing decode is the point
            if mod.last_reclaimed():
                vid, bad_buf, root = v, b, r["root"]
                break
    if vid is None:
        return False, "no failing retain decode reclaimed a buffer: the control is blind"
    ok_buf = open(rpath(man[vid]["file"]), "rb").read()
    evA, evB, seen = threading.Event(), threading.Event(), {}

    def a():
        try:
            mod.decode("cext", root, bad_buf, ty, None, True)
        except Exception:  # noqa: BLE001
            pass
        seen["n1"] = mod.last_reclaimed()
        evA.set()
        evB.wait(10)
        seen["n2"] = mod.last_reclaimed()

    def b():
        evA.wait(10)
        mod.decode("cext", root, ok_buf, ty, None, True)
        seen["b"] = mod.last_reclaimed()
        evB.set()
    ts = [threading.Thread(target=a), threading.Thread(target=b)]
    for t in ts:
        t.start()
    for t in ts:
        t.join()
    ok = seen.get("n1", 0) > 0 and seen.get("n2") == seen.get("n1") and seen.get("b") == 0
    return ok, ("%s, %s: thread A's failed decode reclaimed %s, thread B's successful decode %s, A reads %s after B"
                % (label, vid, seen.get("n1"), seen.get("b"), seen.get("n2")))


def nounk_controls(out=sys.stdout):
    """The no-unknown variant's controls (WP5 step 10), in one process."""
    arm = Arm("ffi-cext")
    mod = arm.mod
    names = [n[1:] for n in mod.types()]
    ty = tuple(arm.ctors[n] for n in names)
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))["vectors"]
    bad = 0
    roots = sorted(arm.roots)
    npos = sum(len(mod.unk_positions(r)) for r in roots)
    print("   positions: %d over %d roots (want 0: no options exist in this variant)" % (npos, len(roots)), file=out)
    bad += 1 if npos else 0
    rows = sorted(k for k, r in man.items() if r["class"] == "unknown" and r["expect"] == "accept"
                  and r.get("verdict") != "disputed" and r["root"] in arm.roots)
    # No facade class of the variant carries `_unknown` (CAMPAIGN req 10: no capture state in
    # the binding): the Plain/Slots classes, the C extension types, and every object decoded
    # from an unknown row, through both backends.
    import facts as F
    objs = []
    for v in rows:
        r = man[v]
        buf = open(rpath(r["file"]), "rb").read()
        objs.append((mod.decode("cext", r["root"], buf, ty), r["root"]))
        objs.append((mod.decode("attr", r["root"], buf, tuple(getattr(arm.fac, "Plain" + n) for n in names)), r["root"]))
    found = F.unknown_slots(arm.fac, mod, objs)
    print("   _unknown on the variant's facades: %d finding(s) over %d Plain/Slots classes, %d C types and %d decoded "
          "objects%s" % (len(found), 2 * len(arm.fac.MESSAGES), len(mod.types()), len(objs),
                         (": " + "; ".join(found[:5])) if found else ""), file=out)
    bad += len(found)
    # the check must be able to fail: the FULL build's facade, same classes, must show it
    import importlib.util
    spec = importlib.util.spec_from_file_location("full_facade", os.path.join(HERE, "gen", "out", "corpus", "facade.py"))
    ff = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ff)
    nf = len(F.unknown_slots(ff))
    print("   must-fail twin: the full build's corpus facade shows %d finding(s) (want > 0)" % nf, file=out)
    bad += 0 if nf else 1
    refused = 0
    for v in rows:
        r = man[v]
        buf = open(rpath(r["file"]), "rb").read()
        o = mod.decode("cext", r["root"], buf, ty)
        for call in (lambda: mod.decode("cext", r["root"], buf, ty, None, True),
                     lambda: mod.encode("cext", r["root"], o, None, True)):
            try:
                call()
            except ValueError as e:
                if "compiled out" in str(e):
                    refused += 1
    print("   retain refused: %d of %d retain calls (decode and encode, every unknown row)" % (refused, 2 * len(rows)), file=out)
    bad += 0 if refused == 2 * len(rows) and rows else 1
    wr_bad = pos_ok = 0
    for a in roots:
        for b in roots:
            rr, dr, delivered = mod.wrong_root(a, b)
            if a == b:
                pos_ok += 1 if (rr == 0 and dr == 0 and delivered) else 0
                wr_bad += 0 if (rr == 0 and dr == 0 and delivered) else 1
            elif not (rr == 0 and dr == -8 and not delivered):
                wr_bad += 1
                print("   WRONG ROOT ACCEPTED: ctx %s, root %s -> decode %d delivered %s" % (a, b, dr, delivered), file=out)
    print("   wrong root: %d ordered pairs refused with -8 and nothing delivered (no reset exists); "
          "positive control %d of %d roots; %d failure(s)"
          % (len(roots) * (len(roots) - 1) - wr_bad, pos_ok, len(roots), wr_bad), file=out)
    bad += wr_bad
    # the per-thread contexts, drop mode only (the variant has no other)
    import threading
    bufs = [(man[v]["root"], open(rpath(man[v]["file"]), "rb").read()) for v in rows]
    want = [mod.encode("cext", r, mod.decode("cext", r, b, ty)) for r, b in bufs]
    c0 = mod.tls_created()
    for _ in range(64):
        for r, b in bufs:
            mod.encode("cext", r, mod.decode("cext", r, b, ty))
    reuse = mod.tls_created() - c0
    errs = []

    def work():
        try:
            for _ in range(40):
                for (r, b), w in zip(bufs, want):
                    if mod.encode("cext", r, mod.decode("cext", r, b, ty)) != w:
                        errs.append("differs %s" % r)
        except Exception as e:  # noqa: BLE001
            errs.append(repr(e))
    old = sys.getswitchinterval()
    sys.setswitchinterval(1e-6)
    c1 = mod.tls_created()
    ts = [threading.Thread(target=work) for _ in range(8)]
    for t in ts:
        t.start()
    for t in ts:
        t.join()
    sys.setswitchinterval(old)
    made = mod.tls_created() - c1
    nroots = len({r for r, _b in bufs}) + 1
    print("   per-thread contexts: reuse %d decode+encode(s) created %d (want 0); 8 threads x %d: %d error(s), "
          "%d context(s) created (at most %d)" % (64 * len(bufs), reuse, 40 * len(bufs), len(errs), made, 8 * nroots), file=out)
    bad += (1 if reuse else 0) + len(errs) + (1 if made > 8 * nroots or made == 0 else 0)
    print("NOUNK CONTROLS %s" % ("PASS" if not bad else "FAIL (%d)" % bad), file=out)
    return 1 if bad else 0


def tls_control(mod, ty, man, rows, out):
    """Decode contexts are per root per thread (the decision 11 port's second form).
    Reuse: 64 more decode+encodes of every row, drop and retain, on this thread create no
    context (the per-thread decode contexts, one per root, and the one encode context).
    Threads: 8 threads with a 1 us switch interval decode (retain) and re-encode every row
    40 times; every re-encoding equals this thread's, no buffer is reclaimed, and at most one
    context per (thread, root) is created."""
    import threading
    bad = 0
    bufs = [(man[v]["root"], open(rpath(man[v]["file"]), "rb").read()) for v in rows]
    want = [mod.encode("cext", r, mod.decode("cext", r, b, ty, None, True), None, True) for r, b in bufs]
    c0 = mod.tls_created()
    for _ in range(64):
        for r, b in bufs:
            mod.encode("cext", r, mod.decode("cext", r, b, ty, None, False), None, False)
            mod.encode("cext", r, mod.decode("cext", r, b, ty, None, True), None, True)
    reuse = mod.tls_created() - c0
    print("   per-thread contexts, reuse: %d decode+encode(s) on this thread created %d context(s) (want 0)"
          % (128 * len(bufs), reuse), file=out)
    bad += 1 if reuse else 0
    nroots = len({r for r, _b in bufs}) + 1   # + the thread's one encode context
    errs = []
    def work():
        try:
            for _ in range(40):
                for (r, b), w in zip(bufs, want):
                    o = mod.decode("cext", r, b, ty, None, True)
                    if mod.last_reclaimed():
                        errs.append("reclaimed %s" % r)
                    if mod.encode("cext", r, o, None, True) != w:
                        errs.append("differs %s" % r)
        except Exception as e:  # noqa: BLE001 -- reported, counted
            errs.append(repr(e))
    old = sys.getswitchinterval()
    sys.setswitchinterval(1e-6)
    c1 = mod.tls_created()
    ts = [threading.Thread(target=work) for _ in range(8)]
    for t in ts:
        t.start()
    for t in ts:
        t.join()
    sys.setswitchinterval(old)
    made = mod.tls_created() - c1
    print("   per-thread contexts, threads: 8 x %d decode+encode(s); %d error(s); %d context(s) created (at most %d)"
          % (40 * len(bufs), len(errs), made, 8 * nroots), file=out)
    for e in errs[:5]:
        print("   THREAD ERROR %s" % e, file=out)
    bad += len(errs) + (1 if made > 8 * nroots or made == 0 else 0)
    return bad


def main(argv):
    if "--worker" in argv:
        return worker(argv[argv.index("--worker") + 1])
    if "--d11" in argv:
        return nounk_controls() if NOUNK else d11_controls()
    arms = ARMS
    if "--arms" in argv:
        arms = argv[argv.index("--arms") + 1].split(",")
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    timeout = float(argv[argv.index("--timeout") + 1]) if "--timeout" in argv else 20.0
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))
    print("# python slice: the conformance corpus, every row, every arm (FIX-PLAN WP5 item 6.1)")
    print("#   manifest   ffi/corpus/generated/manifest.json (%d rows, corpus %s)"
          % (len(man["vectors"]), man.get("corpus_version", "?")))
    print("#   generated  gen/out/corpus/*, rendered by poc/codec/gen from the corpus READER plan")
    if NOUNK:
        print("#   VARIANT    no-unknown (WP5 step 10): _akffi_corpus_nounk over ak-core --no-default-features "
              "--features corpus,init-guard; unknown fields compiled out")
    else:
        print("#   ffi arms   _akffi_corpus over ak-core --features corpus,init-guard; unknown fields dropped")
    print("#   py arms    pycodec.py (drop), pycodec_retain.py (retain), plain facade")
    print("#   interpreter %s; each row in a worker process, timeout %.0f s" % (sys.version.split()[0], timeout))
    t0 = time.time()
    man, ids, results = run(arms, only, timeout)
    fails, hung = summarise(man, ids, results)
    if "--dump" in argv:
        # Every re-encoding's sha256 per arm, so two interpreter levels can be compared byte
        # for byte (CLAUDE.md: identical wire bytes across levels, checked by the corpus).
        with open(argv[argv.index("--dump") + 1], "w") as fh:
            json.dump({a: {v: results[a][v].get("enc") for v in ids} for a in results}, fh,
                      sort_keys=True)
    if "--compare" in argv:
        other = json.load(open(argv[argv.index("--compare") + 1]))
        print("\n## byte identity against another interpreter level's run")
        nd = nn = 0
        for a in results:
            for v in ids:
                x, y = results[a][v].get("enc"), (other.get(a) or {}).get(v)
                if x or y:
                    nn += 1
                    if x != y:
                        nd += 1
                        print("   DIFFER %s [%s]: %s vs %s" % (v, a, x and x[:12], y and y[:12]))
        print("   %d (arm, row) re-encodings compared with %s; %d differ"
              % (nn, os.path.basename(argv[argv.index("--compare") + 1]), nd))
        fails += nd
    print("\n## between-arm byte identity (CONTRACT 5.5), drop arms")
    bad = cross_arm(ids, results)
    if RETAIN_ARMS:
        print("\n## between-arm byte identity, retain arms (ffi-retain against py-retain)")
        bad += cross_arm(ids, results, family=RETAIN_ARMS)
    print("\n## retain arms: rows where the retained form was NOT written (a retention gap)")
    for a in RETAIN_ARMS:
        if a not in results:
            continue
        gap = [v for v in ids if man["vectors"][v]["class"] == "unknown" and not results[a][v].get("notabi")
               and (results[a][v].get("form") or "").startswith("unknown-dropped")]
        dis = [v for v in ids if results[a][v].get("disputed") and man["vectors"][v]["class"] == "unknown"]
        print("   %-12s %d row(s) wrote the dropped form: %s%s" % (a, len(gap), ", ".join(gap) or "none",
              ("; disputed (excluded, reading reported above): " + ", ".join(dis)) if dis else ""))
    if NOUNK:
        print("\n## no-unknown variant: every unknown-class row writes the DROPPED form on every ffi arm")
        for a in [x for x in results if x.startswith("ffi")]:
            unk = [v for v in ids if man["vectors"][v]["class"] == "unknown" and not results[a][v].get("notabi")
                   and not results[a][v].get("disputed") and man["vectors"][v]["expect"] == "accept"]
            # A row whose manifest offers no dropped form (an open enum's unlisted value is a
            # KNOWN field, kept on decode) is checked by C3 against its one accepted form.
            hasd = [v for v in unk if any(f.startswith("unknown-dropped") for e in man["vectors"][v].get("accepted_encodings", [])
                                          for f in e.get("forms", []))]
            nod = [v for v in unk if v not in hasd]
            notd = [v for v in hasd if not (results[a][v].get("form") or "").startswith("unknown-dropped")]
            print("   %-12s %d unknown row(s) with a dropped form: %d not written in it%s; %d row(s) with no dropped form "
                  "in the manifest (checked by C3 only): %s"
                  % (a, len(hasd), len(notd), (": " + ", ".join(notd)) if notd else "", len(nod), ", ".join(nod) or "none"))
            bad += len(notd)
            if not hasd:
                print("   NO UNKNOWN ROW RAN on %s: the check is blind" % a)
                bad += 1
    print("\n## C4: the refusal each arm returned, per reject vector")
    refusals(ids, man, results)
    print("\n# rows that hung or crashed a worker: %d" % hung)
    print("# wall %.0f s (instrumentation)" % (time.time() - t0))
    ok = not fails and not bad and not hung
    print("CORPUS PASSES on all %d arms" % len(arms) if ok else "CORPUS: %d failure line(s), %d cross-arm difference(s)" % (fails, bad))
    rc = 0 if ok else 1
    if "--controls" in argv:
        print("\n===== controls (each MUST FAIL) =====")
        sub = ["S-Probe", "U-root", "X-lenwrap-lrr", "E-map", "T-dec-root", "B-P2"]
        bad_ctl = 0
        noinit_arms = ["ffi-cext", "ffi-attr"] + ([] if NOUNK else ["ffi-retain"])
        for plant, carms in (("proj", ARMS), ("reenc", ARMS), ("accept", ARMS), ("noinit", noinit_arms)):
            m2, ids2, res2 = run(carms, sub, timeout, plant=plant)
            nf = sum(1 for a in res2 for v in ids2 if res2[a][v].get("fails"))
            per = {a: sum(1 for v in ids2 if res2[a][v].get("fails")) for a in res2}
            allfail = all(per[a] > 0 for a in per)
            if not allfail:
                bad_ctl += 1
                print("  control %-6s: PASSED on %s -- the harness is blind to it"
                      % (plant, ", ".join(a for a in per if not per[a])))
            else:
                ex = next(("%s [%s]: %s" % (v, a, res2[a][v]["fails"][0]) for a in res2 for v in ids2
                           if res2[a][v].get("fails")), "")
                print("  control %-6s: failed as required on every arm (%d arm-row failures over %d rows; %s)"
                      % (plant, nf, len(ids2), ", ".join("%s=%d" % kv for kv in per.items())))
                print("      e.g. %s" % ex[:150])
        print("CONTROLS %s" % ("ALL FAILED AS REQUIRED" if not bad_ctl else "BLIND: %d" % bad_ctl))
        print("\n===== decision 11 controls (zeroed position, wrong root, leak) =====")
        r11 = subprocess.run([sys.executable, os.path.abspath(__file__), "--d11"], capture_output=True, text=True)
        sys.stdout.write(r11.stdout + r11.stderr[-2000:])
        bad_ctl += 1 if r11.returncode else 0
        rc = rc or (1 if bad_ctl else 0)
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
