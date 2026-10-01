#!/usr/bin/env bash
# pertest.sh TREE OUT: build h2-tests, run every integration test alone with a 30 s timeout
set -u
cd "$1"
taskset -c 0,9,10,19 cargo test -q -p h2-tests -j4 --no-run --message-format=json 2>/dev/null \
  | python3 -c 'import sys,json
for l in sys.stdin:
    try: m=json.loads(l)
    except: continue
    if m.get("reason")=="compiler-artifact" and m.get("executable") and m["target"]["kind"]==["test"]: print(m["executable"])' > "$2.bins"
: > "$2"
while read -r b; do
  for t in $("$b" --list 2>/dev/null | sed -n 's/: test$//p'); do
    taskset -c 0,9,10,19 timeout 30 "$b" --exact "$t" -q >/dev/null 2>&1; rc=$?
    echo "$(basename "$b" | sed 's/-[0-9a-f]*$//') $t rc=$rc" >> "$2"
  done
done < "$2.bins"
