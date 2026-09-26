#!/usr/bin/env python3
"""CAMPAIGN req 7 (amended 2026-09-26): the corpus's accepted, non-disputed unknown-class rows
whose root is one of the shapes core's 7 ABI roots (92 as of 2026-09-26), as a TSV of
"id<TAB>root<TAB>file" (file relative to the corpus's generated directory). Glue: it reads
the manifest and nothing else.

  u_rows.py CORPUS_GENERATED_DIR OUT.tsv
"""
import json
import os
import sys

ROOTS = {"ListResultsResponse", "ListTasksDetailedResponse", "ListProbeResponse",
         "ListTaskSummaryResponse", "UploadResultDataMessage", "ListMetricsResponse", "DualResponse"}


def main(d, out):
    m = json.load(open(os.path.join(d, "manifest.json")))["vectors"]
    n = 0
    with open(out, "w") as f:
        for k in sorted(m):
            r = m[k]
            if r.get("class") == "unknown" and r.get("expect") == "accept" and r.get("root") in ROOTS \
                    and r.get("verdict") != "disputed":
                f.write("%s\t%s\t%s\n" % (k, r["root"], r["file"]))
                n += 1
    return 0 if n else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
