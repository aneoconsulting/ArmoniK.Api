#!/usr/bin/env python3
"""WP5 step 6: the probe manifest (gen/probe_corpus.py, at /tmp/claude-0/probecorpus or edit the path) against a GENERATED pure-Python codec directory (the python slice's gen/out/corpus). Usage: probe_pycodec.py DIR"""
import sys, os, json, importlib.util
out = sys.argv[1]; man = json.load(open("/tmp/claude-0/probecorpus/manifest.json"))["vectors"]
sys.path.insert(0, out)
for modname in ("pycodec", "pycodec_retain"):
    spec = importlib.util.spec_from_file_location(modname, os.path.join(out, modname + ".py"))
    m = importlib.util.module_from_spec(spec); sys.modules[modname]=m; spec.loader.exec_module(m)
    import facade as F
    bad = 0
    for rid, r in sorted(man.items()):
        b = open("/tmp/claude-0/probecorpus/" + r["file"], "rb").read()
        try:
            m.decode_root_WireZoo(b, {n: getattr(F, "Plain" + n) for n in F.MESSAGES}); got = "accept"
        except Exception as e:
            got = "reject(%s)" % getattr(e, "code", type(e).__name__)
        ok = got.startswith(r["expect"])
        bad += not ok
        print("%-14s %-28s expect %-6s got %-18s %s" % (modname, rid, r["expect"], got, "ok" if ok else "WRONG"))
    print("%s: %d wrong" % (modname, bad))
