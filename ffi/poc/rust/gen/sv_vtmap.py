#!/usr/bin/env python3
# Static decode vtables (owner, 2026-10-04). For each C++ binding: the old per-call assignments
# (at BASE) mapped member -> function, the header's member order, and the new positional
# aggregate: the new one must assign every member the same function as before.
# Usage: gen/sv_vtmap.py [BASE] (default e0586a85^, the parent of the poc(codec) commit that made
# the vtables static).
import os, re, subprocess, sys
BASE = sys.argv[1] if len(sys.argv) > 1 else "e0586a85^"
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "cpp"))
pairs = [("src/generated/binding.cpp","include/ak_abi.h"),("src/generated/binding_borrow.cpp","include/ak_abi.h"),
         ("src/generated/binding_nounk.cpp","nounk/include/ak_abi.h"),("src/generated/binding_borrow_nounk.cpp","nounk/include/ak_abi.h"),
         ("corpus/src/generated/binding.cpp","corpus/include/ak_abi.h"),("corpus/src/generated/binding_nounk.cpp","corpus/nounk/include/ak_abi.h")]
bad = 0
n = 0
for b, h in pairs:
    old = subprocess.check_output(["git","show","%s:ffi/poc/cpp/%s" % (BASE, b)]).decode()
    new = open(b).read(); hdr = open(h).read()
    roots = re.findall(r"struct ak_dvt_(\w+) vt;", old)
    for r in roots:
        blk = old[old.index("struct ak_dvt_%s vt;" % r):]
        blk = blk[:blk.index("return ak_decode_")]
        om = dict(re.findall(r"vt\.(\w+) = (\w+);", blk))
        sb = hdr[hdr.index("struct ak_dvt_%s {" % r):]; sb = sb[:sb.index("};")]
        mem = re.findall(r"^\s+(?:uint64_t (\w+);|.*?\(\*(\w+)\)\()", sb, re.M)
        mem = [a or c for a, c in mem]
        nb = new[new.index("static const struct ak_dvt_%s k_dvt_%s = {" % (r, r)):]
        nb = nb[nb.index("{")+1:nb.index("};")]
        vals = [re.sub(r"/\*.*?\*/", "", v).strip() for v in re.sub(r"/\*.*?\*/", "", nb).split(",")]
        nm = dict(zip(mem, vals))
        ok = len(mem) == len(vals) and all(om.get(k) == ("0" if k == "utf8_skip" else nm[k]) or (k=="utf8_skip" and nm[k]=="0") for k in mem) and set(om) == set(mem)
        bad += not ok
        n += 1
        print("%-6s %-42s %-30s members %2d old %2d new %2d" % ("ok" if ok else "DIFF", b, r, len(mem), len(om), len(vals)))
print("vtables compared against %s: %d, mismatches: %d" % (BASE, n, bad)); sys.exit(1 if bad else 0)
