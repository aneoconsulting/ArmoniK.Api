#!/usr/bin/env python3
"""FIX-PLAN WP5 item 6.1: the FULL conformance corpus (`ffi/corpus/CONTRACT.md`) through the
cpp slice's C ABI binding and core-native, unknown fields dropped and retained. Glue: it
spawns the C++ child (`corpus/src/corpus_main.cpp`) once per row under a timeout and takes
the verdict. Modelled on `poc/rust/corpus/crates/harness/src/main.rs`, the same rules:

  C1  every accept row decodes;
  C2  its projection equals the committed one (`_unknown` stripped: optional per the contract);
  C3  the re-encoding is one of `accepted_encodings`, or a re-ordering of the committed
      records where `permutation_accepted`; the FORM written is recorded;
  C4  every reject row is refused (the code is recorded).
  A `disputed` row is excluded from pass and fail and the reading produced is reported.
  A retain arm that wrote the unknown-DROPPED form on an `unknown` row is accepted by the
  contract and listed as a retention gap.

Arms: ffi-drop, ffi-retain (the core built `--features corpus,init-guard`, the binding
rendered from the corpus plan), native-drop, native-retain (core-native rendered from the
same plans). A root the C ABI refuses (plan.check_expressible) is NOT IN THE ABI for its ffi
arms, by name.

  corpus_all.py BINARY [--manifest PATH] [--only P1,P2] [--timeout S] [--plant proj|reenc|accept]
                       [--expect-fail]   exit 0 iff the run FAILED (a control)
                       [--max-retain-gap ID,...]  fail if a retain arm writes the dropped
                                         form on any row not listed
                       [--record FILE]   write every (row, arm) outcome, so two builds (two
                                         standard levels, two linkages) can be compared
                                         byte for byte: `corpus_all.py --compare A B`
  corpus_all.py BINARY --unk-controls [--manifest PATH] [--only ...] [--plant clear] [--expect-fail]
                       decision 11's controls on every row the C ABI carries: pool decode
                       equal to retain, drop decode equal to retain with every bag cleared,
                       each position zeroed in turn dropping exactly that position, map-entry
                       bytes delivered unless the entry position is zeroed. `--plant clear`
                       skips the expected clear (the control seen failing).
"""
import concurrent.futures as cf
import hashlib
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SLICE = os.path.dirname(HERE)
FFI = os.path.dirname(os.path.dirname(SLICE))
CORPUS = os.path.join(FFI, "corpus", "generated")
ARMS = ["ffi-drop", "ffi-retain", "native-drop", "native-retain"]


def strip_unknown(v):
    if isinstance(v, dict):
        return {k: strip_unknown(x) for k, x in v.items() if k != "_unknown"}
    if isinstance(v, list):
        return [strip_unknown(x) for x in v]
    return v


def records(b):
    """Top-level wire records (key + value bytes), sorted: the permutation rule."""
    out, i = [], 0
    n = len(b)

    def varint(i):
        v, s = 0, 0
        while True:
            if i >= n or s > 63:
                return None, i
            c = b[i]
            i += 1
            v |= (c & 0x7f) << s
            if not c & 0x80:
                return v, i
            s += 7

    while i < n:
        s0 = i
        k, i = varint(i)
        if k is None:
            return None
        w = k & 7
        if w == 0:
            v, i = varint(i)
            if v is None:
                return None
        elif w == 1:
            i += 8
        elif w == 2:
            ln, i = varint(i)
            if ln is None:
                return None
            i += ln
        elif w == 5:
            i += 4
        else:
            return None
        if i > n:
            return None
        out.append(bytes(b[s0:i]))
    return sorted(out)


def run_child(binary, row, timeout, plant, unk=False):
    env = dict(os.environ)
    if plant and not unk:
        env["AK_CORPUS_PLANT"] = plant
    if plant == "clear" and unk:
        env["AK_CORPUS_UNK_PLANT"] = "1"
    path = os.path.join(CORPUS, row["file"])
    try:
        p = subprocess.run([binary] + (["--unk"] if unk else []) + [path, row["root"]], stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout, env=env)
    except subprocess.TimeoutExpired:
        return {"timeout": timeout}
    if p.returncode != 0:
        err = p.stderr.decode("utf-8", "replace").strip().splitlines()
        return {"crash": "exit %d %s" % (p.returncode, err[-1] if err else "")}
    try:
        return json.loads(p.stdout.decode("utf-8"))
    except ValueError as e:
        return {"crash": "bad child output: %s" % e}


class Tally:
    def __init__(self):
        self.passed = self.failed = self.disputed = self.na = self.not_built = 0
        self.forms = {}
        self.fails = []
        self.disputes = []
        self.retain_gap = []
        self.by_class = {}
        self.refusals = {}


def eval_arm(row, rid, arm, r, vec, t):
    cls = row.get("class", "?")
    if r.get("na"):
        t.na += 1
        return
    if r.get("not_built"):
        t.not_built += 1
        return
    disputed = row.get("verdict") == "disputed"
    expect = row["expect"]
    why = []
    if "timeout" in r or "crash" in r or r.get("unknown_root"):
        why.append("child %s" % json.dumps(r))
    elif expect == "reject":
        if r.get("ok"):
            why.append("C4: accepted a must-fail vector")
        else:
            t.refusals[r.get("err")] = t.refusals.get(r.get("err"), 0) + 1
    elif not r.get("ok"):
        why.append("C1: refused with %s" % r.get("err"))
    else:
        pf = row.get("projection")
        if pf:
            want = strip_unknown(json.load(open(os.path.join(CORPUS, pf))))
            if want != r.get("proj"):
                why.append("C2: projection differs: got %s want %s"
                           % (json.dumps(r.get("proj"))[:300], json.dumps(want)[:300]))
        h = r.get("hex")
        if h is None:
            why.append("C3: re-encode failed: %s" % r.get("enc_err"))
        else:
            re = bytes.fromhex(h)
            s = hashlib.sha256(re).hexdigest()
            hit = [f for f in row.get("accepted_encodings") or [] if f["sha256"] == s]
            if hit:
                label = " / ".join(hit[0].get("forms", []))
                t.forms[label] = t.forms.get(label, 0) + 1
                if arm.endswith("retain") and cls == "unknown" and "dropped" in label:
                    t.retain_gap.append("%s (%s)" % (rid, arm))
            else:
                rr = records(re)
                if row.get("permutation_accepted") and rr is not None and rr == records(vec):
                    k = "a re-ordering of the committed form (permutation_accepted)"
                    t.forms[k] = t.forms.get(k, 0) + 1
                else:
                    why.append("C3: re-encoding %s (%d B) is not an accepted form" % (s[:12], len(re)))
    if disputed:
        t.disputed += 1
        if not r.get("ok"):
            reading = "refused (%s)" % r.get("err")
        else:
            which = "none of the readings"
            for rd in (row.get("dispute") or {}).get("readings") or []:
                pf = rd.get("projection")
                if pf and strip_unknown(json.load(open(os.path.join(CORPUS, pf)))) == r.get("proj"):
                    which = "the reading of %s" % json.dumps(rd.get("runtimes"))
            reading = "accepted; projection matches %s" % which
        t.disputes.append("%s [%s]: %s" % (rid, arm, reading))
        return
    c = t.by_class.setdefault(cls, [0, 0])
    if not why:
        t.passed += 1
        c[0] += 1
    else:
        t.failed += 1
        c[1] += 1
        t.fails.append("%s [%s]: %s" % (rid, arm, "; ".join(why)))


def unk_controls(binary, ids, rows, timeout, plant, expect_fail):
    """Decision 11's controls (see the module doc), one child per row."""
    print("# decision 11 controls through the C ABI binding (corpus_all --unk), %d rows" % len(ids))
    if plant:
        print("#   PLANTED DEFECT: %s (a control run: it MUST fail)" % plant)
    with cf.ThreadPoolExecutor(max_workers=4) as ex:
        results = dict(zip(ids, ex.map(lambda i: run_child(binary, rows[i], timeout, plant, unk=True), ids)))
    n = na = refused = with_unk = entry_rows = positions = changed = refills = 0
    bad = []
    for rid in ids:
        r = results[rid]
        if r.get("na"):
            na += 1
            continue
        n += 1
        if "timeout" in r or "crash" in r:
            bad.append("%s: %s" % (rid, json.dumps(r)))
            continue
        if "err" in r:
            refused += 1
            if not (r["err"] == r["drop_err"] == r["pool_err"]):
                bad.append("%s: refusal differs across modes %s" % (rid, json.dumps(r)))
            continue
        positions += r["positions"]
        changed += r["changed"]
        refills += r["refills"]
        with_unk += 1 if r["changed"] else 0
        entry_rows += 1 if r["entry_bytes"] else 0
        why = []
        if r["mismatched"]:
            why.append("zeroed position(s) %s did not drop exactly that position" % r["mismatched"])
        if r["entry_mismatch"]:
            why.append("map-entry bytes wrong with position(s) %s zeroed" % r["entry_mismatch"])
        if not r["pool_equal"]:
            why.append("pool decode differs from retain")
        if not r["drop_equal"]:
            why.append("drop decode differs from retain with every bag cleared")
        if r["leaked"]:
            why.append("%d buffer(s) left live after the decodes" % r["leaked"])
        if why:
            bad.append("%s: %s" % (rid, "; ".join(why)))
    print("   rows through the C ABI %d (not in the ABI %d); refused by every mode alike %d" % (n, na, refused - sum(1 for b in bad if "refusal differs" in b)))
    print("   positions checked %d; position-rows where zeroing changes the value %d; rows with unknowns %d"
          % (positions, changed, with_unk))
    print("   rows whose map entries carried unknown bytes (delivered, freed: U-map-entry) %d" % entry_rows)
    print("   pool buffers refilled in place across all rows %d" % refills)
    for b in bad[:60]:
        print("   FAIL %s" % b)
    failed = bool(bad)
    print("UNK CONTROLS %s: %d failing row(s)" % ("FAIL" if failed else "PASS", len(bad)))
    if expect_fail:
        return 0 if failed else 1
    return 1 if failed else 0


def main(argv):
    global CORPUS
    args = list(argv)
    if args and args[0] == "--compare":
        return compare(args[1:])
    binary = args.pop(0)
    only, timeout, plant, expect_fail, record, unk, max_gap = [], 10.0, None, False, None, False, None
    while args:
        a = args.pop(0)
        if a == "--unk-controls":
            unk = True
        elif a == "--max-retain-gap":
            max_gap = set(args.pop(0).split(","))
        elif a == "--manifest":
            CORPUS = os.path.dirname(os.path.abspath(args.pop(0)))
        elif a == "--only":
            only = args.pop(0).split(",")
        elif a == "--timeout":
            timeout = float(args.pop(0))
        elif a == "--plant":
            plant = args.pop(0)
        elif a == "--expect-fail":
            expect_fail = True
        elif a == "--record":
            record = args.pop(0)
        else:
            raise SystemExit("unknown argument %s" % a)
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))
    rows = man["vectors"]
    if unk:
        ids = [i for i in sorted(rows) if not only or any(i.startswith(p) for p in only)]
        return unk_controls(binary, ids, rows, timeout, plant, expect_fail)
    info = json.loads(subprocess.run([binary, "--info"], stdout=subprocess.PIPE).stdout)
    print("# the conformance corpus, C ABI and core-native, unknown fields dropped and retained")
    print("#   binary     %s" % os.path.relpath(binary, SLICE))
    print("#   build      __cplusplus %s, target impl %s, binding skips ak_init: %s"
          % (info["cplusplus"], bool(info["cxx17_impl"]), info["skip_init"]))
    print("#   manifest   %s (%d rows)" % (os.path.relpath(os.path.join(CORPUS, "manifest.json"), FFI), len(rows)))
    print("#   core       ak-core --features corpus,init-guard: generated for the corpus's READER")
    print("#              schema by poc/codec/gen (plan.py + rust_abi.py); CONTRACT rule 0")
    print("#   native     rendered by poc/codec/gen/cpp_native.py from the same plans: %s"
          % " / ".join(info["native_unknown"]))
    print("#   decode utf8 policy %s; recursion limit %s; ffi-retain arm built: %s"
          % (info["utf8"], info["limit"], info["ffi_retain_built"]))
    print("#   each row in a child process, timeout %.0f s" % timeout)
    if plant:
        print("#   PLANTED DEFECT: %s (a control run: it MUST fail)" % plant)
    if only:
        print("#   rows limited to ids starting with %s" % only)
    print()
    ids = [i for i in sorted(rows) if not only or any(i.startswith(p) for p in only)]
    tallies = {a: Tally() for a in ARMS}
    hard = []
    with cf.ThreadPoolExecutor(max_workers=4) as ex:
        results = dict(zip(ids, ex.map(lambda i: run_child(binary, rows[i], timeout, plant), ids)))
    if record:
        json.dump({"binary": os.path.basename(binary), "info": info, "rows": results},
                  open(record, "w"), sort_keys=True)
    for rid in ids:
        row = rows[rid]
        res = results[rid]
        vec = open(os.path.join(CORPUS, row["file"]), "rb").read()
        if "timeout" in res or "crash" in res:
            hard.append("%s: %s" % (rid, json.dumps(res)))
            for a in ARMS:
                eval_arm(row, rid, a, res, vec, tallies[a])
            continue
        for a in ARMS:
            eval_arm(row, rid, a, res[a], vec, tallies[a])
    total = 0
    for a in ARMS:
        t = tallies[a]
        total += t.failed
        print("## %s" % a)
        print("   pass %d  fail %d  disputed (excluded) %d  not in the C ABI %d  arm not built %d"
              % (t.passed, t.failed, t.disputed, t.na, t.not_built))
        for c, (p, f) in sorted(t.by_class.items()):
            print("     %-10s pass %4d  fail %3d" % (c, p, f))
        if t.forms:
            print("   forms written (C3):")
            for f, k in sorted(t.forms.items()):
                print("     %4d  %s" % (k, f))
        if t.refusals:
            print("   refusal codes (C4): %s" % ", ".join("%s x%d" % kv for kv in sorted(t.refusals.items(), key=lambda x: str(x[0]))))
        for d in t.disputes:
            print("   disputed: %s" % d)
        if t.retain_gap:
            print("   retain mode wrote the DROPPED form on %d row(s) (accepted by the contract; a retention gap):"
                  % len(t.retain_gap))
            for g in t.retain_gap:
                print("     %s" % g)
        for f in t.fails[:60]:
            print("   FAIL %s" % f)
        if len(t.fails) > 60:
            print("   ... and %d more" % (len(t.fails) - 60))
        print()
    if max_gap is not None:
        # Decision 11 (WP5 step 9): the retain arms may write the dropped form only on the
        # rows named here (the facade's map has no bag: U-map-entry).
        for a in ("ffi-retain", "native-retain"):
            extra = [g for g in tallies[a].retain_gap if g.split(" ")[0] not in max_gap]
            print("## %s retention gaps outside {%s}: %d" % (a, ",".join(sorted(max_gap)), len(extra)))
            for g in extra:
                print("   FAIL retention gap %s" % g)
            total += len(extra)
    print("# rows that hung or crashed a child: %d" % len(hard))
    for h in hard[:20]:
        print("!! %s" % h)
    failed = total > 0 or bool(hard)
    if failed:
        print("CORPUS FAILS: %d arm-row failure(s), %d hang/crash row(s)" % (total, len(hard)))
    else:
        print("CORPUS PASSES on every built arm")
    if expect_fail:
        return 0 if failed else 1
    return 1 if failed else 0


def compare(files):
    """Every (row, arm) outcome of each recorded build against the first: identical
    refusal codes, projections and re-encoded bytes, or a named difference."""
    base = json.load(open(files[0]))
    bad = 0
    for f in files[1:]:
        other = json.load(open(f))
        n = diff = 0
        for rid, arms in base["rows"].items():
            o = other["rows"].get(rid)
            for a in ARMS:
                n += 1
                if o is None or (arms.get(a) if isinstance(arms, dict) else arms) != \
                        (o.get(a) if isinstance(o, dict) else o):
                    diff += 1
                    if diff <= 10:
                        print("  DIFF %s [%s]: %s vs %s" % (rid, a, base["binary"], other["binary"]))
        print("%s vs %s: %d (row, arm) outcomes compared, %d differ"
              % (base["binary"], other["binary"], n, diff))
        bad += diff
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
