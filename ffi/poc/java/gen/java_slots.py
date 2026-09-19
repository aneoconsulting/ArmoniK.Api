"""The one enumeration of vtable slots, shared by the C shim and the Java binding.

Both sides index the same trampoline tables, so if either rebuilt the enumeration
separately a permutation at constant count would be a wrong callback on the right slot --
the cpp slice's defect C5, which a count check cannot see. One list, two readers.
"""
import ir as IR
import rust_abi as RA


def enc_slots(ir):
    """[(vtable message, slot path, field)] in vtable order, globally numbered."""
    out = []
    for name in RA.vtable_messages(ir):
        for path, f in IR.loop_slots(ir, name):
            out.append((name, path, f))
    return out


def enc_vtable(ir, name):
    """The evt struct's members for `name`, as ('loop'|'elem', slot name, element type)."""
    rows = []
    slots = IR.loop_slots(ir, name)
    if not slots:
        return [("reserved", "_reserved", None)]
    for path, f in slots:
        sn = IR.slot_name(path)
        rows.append(("loop", sn, None))
        et = RA.elem_type(f)
        if et and IR.loop_slots(ir, et):
            rows.append(("elem", sn, et))
    return rows


def dec_vtable(ir, name):
    """The dvt struct's members for `name`, as (kind, slot name, element type).

    kind is one of: apply, unknown, unk, add, new, applyelem, addinner. The order is
    exactly `rust_abi`'s, which is the order the core reads.
    """
    rows = [("apply", "", None), ("unknown", "", None)]
    for path, f in IR.loop_slots(ir, name):
        sn = IR.slot_name(path)
        et = RA.elem_type(f)
        batchable = not (et and not ir.msg(et).leaf)
        if et:
            rows.append(("unk", sn, et))
        if batchable:
            rows.append(("add", sn, et))
        else:
            rows.append(("new", sn, et))
            rows.append(("applyelem", sn, et))
            for ipath, iff in IR.loop_slots(ir, et):
                rows.append(("addinner", "%s_%s" % (sn, IR.slot_name(ipath)), iff))
    return rows


def dec_slots(ir):
    """Every decode callback that needs a trampoline, globally numbered, as
    (vtable message, kind, slot name, field-or-None)."""
    out = []
    # ROOTS only. A decode vtable is passed to `ak_decode_<root>` and nowhere else: a
    # non-root element's callbacks live in the ROOT's vtable, flattened, which is what
    # `addinner` is. Enumerating every vtable message here would emit trampolines for a
    # `ak_dvt_TaskDetailed` nothing ever passes -- dead code that still has to be right.
    for name in ir.roots:
        for kind, sn, et in dec_vtable(ir, name):
            if kind == "unknown" or kind == "unk":
                continue          # decision 11's bag is not built here
            out.append((name, kind, sn, et))
    return out


def counts(ir):
    e = len(enc_slots(ir))
    d = {}
    for _n, kind, _sn, _et in dec_slots(ir):
        d[kind] = d.get(kind, 0) + 1
    return e, d
