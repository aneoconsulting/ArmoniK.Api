"""ABI v1 section 7.1's PULL family, emitted for the JVM.

The push family has the codec call the host once per field group; on this runtime that is
7.004 upcalls per element at about 80 ns, which is 560 ns of a 1.441 decode ratio and the
whole of why the C ABI loses on decode here. The pull family has the codec deposit the same
handovers as records into the host-owned context, make **no reverse call at all**, and
return; the host reads them afterwards.

Three things this file is careful about, each because getting it wrong would measure
something other than the family.

**The replay calls the push family's own per-slot methods.** `dec<i>` is the same body the
push vtable's trampoline reaches. If the two families deposited through two different
bodies of Java, a difference between the arms would be a difference between two bindings
rather than between two deliveries of one traversal. The rust slice states the same rule
for the same reason.

**The slot numbering is computed from the shared IR, not copied.** `(outer << 16) | inner`
with the root's loop slots 1-based and, for a non-leaf element, its own loop slots 1-based;
a root-level batchable run is the bare index. That rule lives in `poc/codec/gen/rust_abi.py`
and both sides derive it from `IR.loop_slots`, so a permutation cannot appear on one side
only -- the same argument `java_slots.py` makes for the vtable.

**Parse runs under a critical section and the wire is never copied.** A push entry point
makes upcalls, and an upcall inside `GetPrimitiveArrayCritical` is illegal, so the push arm
has to copy the wire into native scratch first. `ak_parse_*` makes no upcall by
construction, so the pull arm hands the core the host's own array and spans resolve against
that array directly (7.4). That copy is one of the two things the family is supposed to
save, and it is saved here rather than argued for.
"""
import ir as IR
import rust_abi as RA

AK_BDR_APPLY = 1
AK_BDR_ADD = 2
AK_BDR_NEW = 3
AK_BDR_APPLY_ELEM = 4

# 24 bytes: u32 op, u32 slot, i64 token, u32 n, u32 bytes. Section 10's layout argument
# applies to this struct as much as to a group, and `Layout.assertAgreement` checks it.
REC_BYTES = 24


def records(ir, root, dec_ix):
    """[(op, bdr slot, dec slot index, kind)] for one root, in the ABI's numbering."""
    out = [(AK_BDR_APPLY, 0, dec_ix[(root, "apply", "")], "apply")]
    for j, (path, f) in enumerate(IR.loop_slots(ir, root)):
        sn = IR.slot_name(path)
        et = RA.elem_type(f)
        if et and not ir.msg(et).leaf:
            out.append((AK_BDR_NEW, (j + 1) << 16, dec_ix[(root, "new", sn)], "new"))
            out.append((AK_BDR_APPLY_ELEM, (j + 1) << 16,
                        dec_ix[(root, "applyelem", sn)], "applyelem"))
            for k, (ipath, _iff) in enumerate(IR.loop_slots(ir, et)):
                isn = "%s_%s" % (sn, IR.slot_name(ipath))
                out.append((AK_BDR_ADD, ((j + 1) << 16) | (k + 1),
                            dec_ix[(root, "addinner", isn)], "addinner"))
        else:
            out.append((AK_BDR_ADD, j + 1, dec_ix[(root, "add", sn)], "add"))
    return out


def emit(ir, dec_ix, o):
    o.append("")
    o.append("  // ---- ABI v1 7.1, the pull family ---------------------------------------")
    o.append(STATE)
    for root in ir.roots:
        recs = records(ir, root, dec_ix)
        o.append("")
        o.append("  /** Parse into the context making ZERO upcalls, then replay the records.")
        o.append("   *  The wire is handed to the core under a critical section rather than")
        o.append("   *  copied into native scratch, which a push entry point cannot do. */")
        o.append("  public %s parse%s(byte[] wire, int off, int len) {" % (root, root))
        o.append("    lastHostError = null;")
        o.append("    // Neither `decErrReset` nor `bdrReset`: `ak_parse_*` clears the")
        o.append("    // sticky slot AND resets the record buffer at entry. Calling either")
        o.append("    // here would be a forward crossing per decode for nothing.")
        o.append("    check(ak.NativeEntry.parse%s(this, decCtx, wire, off, len));" % root)
        o.append("    %s r = new %s();" % (root, root))
        o.append("    decRoot = r;")
        o.append("    decTokN = 0;")
        o.append("    // 7.4: spans resolve against the array the host already holds. The")
        o.append("    // core parsed THAT array, not a copy of it.")
        o.append("    wireHeap = wire;")
        o.append("    wireBase = off;")
        o.append("    if (pullWalk) {")
        o.append("      // `ak_bdr_ptr`: read the records in place. One forward crossing for")
        o.append("      // the whole response and no intermediate at all. A JVM host can do")
        o.append("      // this because it reads off-heap through Unsafe without pinning.")
        o.append("      check(Native.bdrPtr(decCtx, pullBox));")
        o.append("      replay%s(pullBox[0], pullBox[0] + pullBox[1]);" % root)
        o.append("    } else {")
        o.append("      // `ak_bdr_drain`: 32 KB chunks into memory the host owns, which is")
        o.append("      // what 7.1 specifies. One forward crossing per chunk, and the copy")
        o.append("      // is the difference between the two arms.")
        o.append("      pullCur[0] = 0;")
        o.append("      for (;;) {")
        o.append("        long got = Native.bdrDrain(decCtx, pullChunk, pullChunkCap, pullCur);")
        o.append("        if (got < 0) throw new IllegalStateException(\"core returned \" + got);")
        o.append("        if (got == 0) break;")
        o.append("        replay%s(pullChunk, pullChunk + got);" % root)
        o.append("      }")
        o.append("    }")
        o.append("    return r;")
        o.append("  }")
        o.append("")
        o.append("  /** A record stream IS the call sequence push would have made, so this")
        o.append("   *  dispatches to the same `dec<i>` the push vtable reaches. */")
        o.append("  private void replay%s(long p, long end) {" % root)
        o.append("    while (p < end) {")
        o.append("      int op = Mem.U.getInt(p);")
        o.append("      int slot = Mem.U.getInt(p + 4);")
        o.append("      long token = Mem.U.getLong(p + 8);")
        o.append("      int n = Mem.U.getInt(p + 16);")
        o.append("      long body = p + %d;" % REC_BYTES)
        o.append("      p = body + Mem.U.getInt(p + 20);")
        o.append("      switch (op) {")
        for op, label, args in ((AK_BDR_APPLY, "AK_BDR_APPLY", "body"),
                                (AK_BDR_ADD, "AK_BDR_ADD", "token, body, n"),
                                (AK_BDR_NEW, "AK_BDR_NEW", ""),
                                (AK_BDR_APPLY_ELEM, "AK_BDR_APPLY_ELEM", "token, body")):
            rows = [r for r in recs if r[0] == op]
            if not rows:
                continue
            o.append("        case %d:   // %s" % (op, label))
            o.append("          switch (slot) {")
            for _op, bslot, ix, _kind in rows:
                o.append("            case %d: dec%d(%s); break;" % (bslot, ix, args))
            o.append("            default: throw abi(op, slot);")
            o.append("          }")
            o.append("          break;")
        o.append("        default: throw abi(op, slot);")
        o.append("      }")
        o.append("    }")
        o.append("  }")


STATE = '''
  /** Read the records in place through `ak_bdr_ptr` instead of draining them. The two
   *  differ by exactly the drain copy, which is what the C# slice estimated at 12 to 19
   *  percent of a parse and what this arm measures instead of estimating. */
  public boolean pullWalk = false;

  /** `ak_bdr_ptr` writes {base, len} here; `ak_bdr_drain` keeps its cursor in `pullCur`.
   *  Instance state, never static: two bindings decode on two threads (R12). */
  final long[] pullBox = new long[2];
  final long[] pullCur = new long[1];

  /** The drain's destination. ABI v1 section 7.1 says 32 KB chunks and the core refuses
   *  less than one arena plus one header, so this is that minimum: a host that wanted the
   *  whole response materialised at once would size it from `ak_bdr_footprint`, which is
   *  a different arm and not the one the specification describes. 8-aligned because a
   *  record's payload is an `ak_dfix_*` carrying `long` and `double`. */
  static final int PULL_CHUNK = 32 * 1024 + 24;
  long pullChunk = Mem.alloc(PULL_CHUNK);
  final long pullChunkCap = PULL_CHUNK;

  /** A record for a slot this host does not know is a GENERATOR disagreement, not wire
   *  input, so it fails the operation rather than being skipped the way an unknown tag is.
   *  The rust slice's replay says the same thing through `ak_fail`. */
  static IllegalStateException abi(int op, int slot) {
    return new IllegalStateException("ABI: no host slot for record op=" + op + " slot=" + slot);
  }

  /** What the last parse deposited, for the crossing count and for a host that wants to
   *  bound what it is holding (section 7.1's stated reason for the call existing). */
  public long bdrFootprint() { return Native.bdrFootprint(decCtx); }
'''
