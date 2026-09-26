"""Java backend: the byte offsets of the plan's groups, for a host with no C compiler.

FIX-PLAN WP5 step 3. A Java binding writes a group into off-heap memory at offsets, so it
needs numbers a C host gets from its compiler. What a group CONTAINS -- its members, their
abi types and their order -- is the plan's (`group_fields`, `ugroup_fields`, the trailing
`presence: u32`, `abi_order_topo`), and the order of the layout facts is the one
`cpp_layout.facts` enumerates for the core's run-time export. The one thing rendered here is
what a C compiler does with that list on this platform: the SysV x86-64 size and alignment
of the plan's abi vocabulary (`LEAF`) and C's struct rule (align each member, round the size
to the largest alignment). That is the "spelling" of a group in a language whose structs are
offsets; no member is added, dropped, reordered or re-typed.

Two checks keep it honest, and both are rendered here:
  * run time: `Layout.assertAgreement()` compares every offset with the core's own
    `ak_layout_facts` (ABI v1 section 10), naming the fact that disagrees;
  * compile time: the C header the JNI shim compiles against (`c_abi.emit`, with these
    numbers appended by `java_backend.offset_asserts`) carries a static assertion of every
    one of them against the C compiler's, so the shim does not build if this engine is wrong.
The fixed structs (`ak_str`, `ak_span`, `ak_blob`, `ak_unk_buf`, `ak_unk_opts`,
`ak_unk_pool`) are laid out from `plan.FIXED.structs` by the same rule and checked against
`plan.FIXED.sizes` at generation (R-H15); only C's primitive sizes are this module's.
The pre-WP5 `poc/java/gen/java_layout.py` re-derived the member list itself from
`rust_abi.group_fields` and patched the u-group by string replacement; it retires.
"""
from plan import (FIXED, abi_order_topo, group_fields, ugroup_fields, unk_opts_layout,
                  unk_opts_name, unknown_compiled_out)
import cpp_layout
import java_names as N

WHO = "java_layout.py"

# (size, align) on the SysV x86-64 C ABI of the plan's primitive types. Every struct --
# the fixed ones of plan.FIXED and the groups -- is laid out from these by C's rule.
LEAF = {
    "i32": (4, 4), "i64": (8, 8), "u8": (1, 1), "f64": (8, 8), "u32": (4, 4),
    "u64": (8, 8), "usize": (8, 8), "ptr": (8, 8),
}

# The fixed structs a group or an options struct is built from (plan.FIXED.structs).
FIXED_USED = ["ak_str", "ak_span", "ak_blob", "ak_unk_buf", "ak_unk_opts", "ak_unk_pool"]


def _fixed_type(ty):
    """A plan.FIXED member type as this module's vocabulary: pointers, optional function
    pointers and named function types are `ptr`; a fixed struct is itself."""
    if ty.startswith("*") or ty.endswith("?") or ty.endswith("_fn"):
        return "ptr"
    return ty


class Layout(object):
    def __init__(self):
        self.groups = {}        # struct name -> (size, align, [(member, offset, abi type)])

    def add(self, name, members):
        off, align = 0, 1
        out = []
        for mname, ty in members:
            if ty in LEAF:
                sz, al = LEAF[ty]
            else:
                sz, al = self.groups[ty][0], self.groups[ty][1]
            off = (off + al - 1) // al * al
            out.append((mname, off, ty))
            off += sz
            align = max(align, al)
        size = (off + align - 1) // align * align
        self.groups[name] = (size, align, out)
        return self.groups[name]

    def size(self, sname):
        return self.groups[sname][0]

    def offset(self, sname, member):
        for m, off, _ in self.groups[sname][2]:
            if m == member:
                return off
        raise KeyError("%s.%s" % (sname, member))


def group_members(m, pre):
    """The plan's member list of `ak_<pre>fix_<M>`, with the trailing presence word."""
    fields = ugroup_fields(m) if pre == "u" else group_fields(m, pre != "d")
    return list(fields) + [("presence", "u32")]


def build(p):
    """The groups of `p`. The no-unknown variant (plan: THE NO-UNKNOWN VARIANT) has no
    u-groups and no per-root options struct; its decode groups have no `unknown` member
    (plan.group_fields). The fixed entry types stay (plan.FIXED declares them in both)."""
    nounk = unknown_compiled_out(p)
    lay = Layout()
    fixed = {n: ms for n, _doc, ms in FIXED.structs}
    for n in FIXED_USED:
        size = lay.add(n, [(m, _fixed_type(t)) for m, t in fixed[n]])[0]
        if size != FIXED.sizes[n]:
            raise ValueError("java_layout: %s laid out at %d bytes, plan.FIXED.sizes says %d"
                             % (n, size, FIXED.sizes[n]))
    for name in abi_order_topo(p):
        m = p.msg(name)
        for pre in (("e", "d") if nounk else ("e", "d", "u")):
            lay.add("ak_%sfix_%s" % (pre, name), group_members(m, pre))
    # Decision 11 (WP5 steps 7-9): each root's options struct, in the plan's order
    # (plan.unk_opts_layout), over the two fixed entry types laid out above. A Java binding
    # writes the options into native memory at these numbers; the header static-asserts them.
    for root in ([] if nounk else p.roots):
        lay.add(unk_opts_name(root), [("host", "ptr")] + [(mn, ty) for mn, _m, ty in unk_opts_layout(p, root)])
    return lay


def const(sname, member=None):
    up = sname.upper()
    return "%s_SIZE" % up if member is None else "%s__%s" % (up, member.upper())


def emit_java(p, ns):
    lay = build(p)
    facts = cpp_layout.facts(p)
    o = [N.java_head(p, WHO), "package %s;" % ns, "",
         "/** Group sizes and member offsets: the plan's member lists laid out by the SysV",
         " *  x86-64 C rules, and asserted at load against what the core exports (ABI v1",
         " *  section 10). A Java binding writes into off-heap memory at these numbers. */",
         "public final class Layout {",
         "  private Layout() {}",
         ""]
    for sname, (size, align, members) in lay.groups.items():
        o.append("  public static final int %s = %d;   // align %d" % (const(sname), size, align))
        for mname, off, _ty in members:
            o.append("  public static final int %s = %d;" % (const(sname, mname), off))
        o.append("")
    o.append("  /** The same facts the core exports, in the core's order (cpp_layout.facts). */")
    o.append("  public static final int[] HOST = {")
    names = []
    for sname, members in facts:
        o.append("    %d," % lay.size(sname))
        names.append("sizeof " + sname)
        for mem in members:
            o.append("    %d," % lay.offset(sname, mem))
            names.append("%s.%s" % (sname, mem))
    o.append("  };")
    o.append("")
    o.append("  public static final String[] NAMES = {")
    for nm in names:
        o.append('    "%s",' % nm)
    o.append("  };")
    o.append("")
    o.append(ASSERT)
    o.append("}")
    o.append("")
    return "\n".join(o)


ASSERT = """  private static boolean checked;

  /**
   * ABI v1 section 10 and obligation 12.3, exercised: the core's own `size_of` and
   * `offset_of` for every group against the table above. Once per process per package.
   * `gen/layout_break.sh` perturbs one fact and shows this firing.
   */
  public static synchronized void assertAgreement() {
    if (checked) return;
    int[] core = new int[HOST.length];
    int have = ak.Native.layoutFacts(core);
    if (have != HOST.length)
      throw new IllegalStateException(
          "ABI v1 section 10: the core exports " + have + " layout facts and this host"
          + " reproduced " + HOST.length + ". The two sides are not describing the same"
          + " set of groups (is this the core generated from the same description?).");
    int bad = 0;
    StringBuilder sb = null;
    for (int i = 0; i < have; i++) {
      if (core[i] == HOST[i]) continue;
      if (sb == null) sb = new StringBuilder("ABI v1 section 10: group layout disagreement\\n");
      if (++bad <= 12)
        sb.append("  ").append(NAMES[i]).append(": core ").append(core[i])
          .append(", host ").append(HOST[i]).append('\\n');
    }
    if (sb != null) {
      sb.append("  (").append(bad).append(" of ").append(have).append(" facts disagree)");
      throw new IllegalStateException(sb.toString());
    }
    checked = true;
  }
"""
