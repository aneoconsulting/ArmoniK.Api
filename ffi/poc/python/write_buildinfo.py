"""build.sh helper: what every campaign log header prints as the build (CAMPAIGN.md
requirements 6 and 27). Usage: write_buildinfo.py <out.json> <core dir> <cflags>"""
import json
import os
import re
import subprocess
import sys

out, core, cflags = sys.argv[1], sys.argv[2], sys.argv[3]


def run(*a):
    try:
        return subprocess.run(a, capture_output=True, text=True).stdout.strip().splitlines()[0]
    except Exception as e:  # noqa: BLE001
        return "unknown (%s)" % e


prof = ""
m = re.search(r"^\[profile\.release\]\n(.*?)(?=^\[|\Z)", open(os.path.join(core, "Cargo.toml")).read(),
              re.S | re.M)
if m:
    prof = "; ".join(l.strip() for l in m.group(1).splitlines() if l.strip() and not l.strip().startswith("#"))
json.dump({
    "cc": run("cc", "--version"),
    "cflags": cflags,
    "core_linkage": "shared (libak_core.so through the dynamic linker)",
    "core_profile_release": prof or "cargo default",
    "rustc": run("rustc", "--version"),
    "core_features": {"_akffi": "init-guard", "_akffi_count": "count,init-guard",
                      "_akffi_rpc": "rpc,init-guard", "_akffi_corpus": "corpus,init-guard",
                      # WP5 step 10: the no-unknown variant, --no-default-features (unknown-fields OFF)
                      "_akffi_nounk": "--no-default-features init-guard",
                      "_akffi_count_nounk": "--no-default-features count,init-guard",
                      "_akffi_rpc_nounk": "--no-default-features rpc,init-guard",
                      "_akffi_corpus_nounk": "--no-default-features corpus,init-guard"},
    "core_source": core,
    "snapshot": os.environ.get("AK_SNAPSHOT", "none (working tree)"),
}, open(out, "w"), indent=1, sort_keys=True)
