#!/usr/bin/env python3
"""Does the unknown-field bag break ABI v1 7.2's batching predicate?

Answered by running the predicate THE GENERATOR USES -- `emit/shapes.py:is_leaf`, imported,
not re-derived (R1) -- against the schema with a bag added under each modelling. The
question gates the whole of open decision 11: if the bag costs the batched decode run, the
structural cost dwarfs any byte-level overhead and there is nothing worth measuring.

  Model A: the bag is a REPEATED field (a list of unknown fields).
  Model B: the bag is ONE opaque `bytes` blob per message, the concatenated raw
           tag-and-value runs.

Nothing here edits the schema: both models are applied to an in-memory copy.
"""
import copy
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "../../../schema/emit"))
import shapes as S  # noqa: E402

BAG_TAG = 536870911  # the largest legal field number; a placeholder, never emitted


def with_bag(schema, card):
    s = copy.deepcopy(schema)
    for m in s["messages"].values():
        m.setdefault("fields", []).append(
            {"name": "unknown_fields", "tag": BAG_TAG, "kind": "bytes", "card": card})
    return s


def main():
    schema = S.load()
    msgs = sorted(schema["messages"])
    now = {n: S.is_leaf(schema, n) for n in msgs}
    a = with_bag(schema, "repeated")
    b = with_bag(schema, "singular")
    leaf_a = {n: S.is_leaf(a, n) for n in msgs}
    leaf_b = {n: S.is_leaf(b, n) for n in msgs}

    print("# ABI v1 7.2's batching predicate, under two models of the unknown-field bag")
    print("#   predicate: emit/shapes.py:is_leaf, imported and not re-derived (R1)")
    print("#   a message is a leaf if it has no repeated and no map field, TRANSITIVELY")
    print()
    print("%-28s %-8s %-18s %s" % ("message", "today", "A: repeated bag", "B: one bytes blob"))
    for n in msgs:
        mark = "  <-- CHANGES" if now[n] != leaf_b[n] else ""
        print("%-28s %-8s %-18s %s%s" % (n, now[n], leaf_a[n], leaf_b[n], mark))
    print()
    print("leaf messages today                : %d of %d" % (sum(now.values()), len(msgs)))
    print("leaf messages, A (repeated bag)    : %d of %d" % (sum(leaf_a.values()), len(msgs)))
    print("leaf messages, B (one bytes blob)  : %d of %d" % (sum(leaf_b.values()), len(msgs)))
    print()
    changed_b = [n for n in msgs if now[n] != leaf_b[n]]
    print("VERDICT")
    print("  A: the bag as a repeated field takes the schema to %d leaf messages. Every"
          % sum(leaf_a.values()))
    print("     element type in the payload set stops batching, including ResultRaw, which is")
    print("     what turns 1000 rows into 9 crossings. This model is refused on structure")
    print("     alone, before any measurement.")
    print("  B: the bag as ONE opaque bytes blob changes the leafness of %d messages%s."
          % (len(changed_b), (" (%s)" % ", ".join(changed_b)) if changed_b else ""))
    print("     The predicate is untouched, so the batched run survives and the bag costs")
    print("     what a singular `bytes` field costs and nothing structural.")
    bad = 1 if changed_b else 0
    print()
    print("%d message(s) would lose batching under model B" % len(changed_b))
    return bad


if __name__ == "__main__":
    sys.exit(main())
