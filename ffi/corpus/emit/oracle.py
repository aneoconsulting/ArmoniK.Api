"""One protobuf runtime's reading of the corpus, in a form another can be diffed against.

    python3 emit/oracle.py <generated-dir>      jobs on stdin as JSON, results on stdout

Importable, and also runnable as a subprocess so that a runtime selected by an
environment variable at import time can be asked the same questions. That is the
only way to get protobuf-python's **pure** backend and its **upb** backend into
one comparison, since `PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION` is read once when
`google.protobuf` is first imported.

Why this file exists at all: the first build of this corpus generated every
vector, every accepted encoding and every projection with upb, and validated
every accept and reject verdict by parsing with upb. **One runtime deciding what
the right answer is makes that runtime the specification**, and on at least one
row it is the minority: on `U-map-entry`, upb drops the entry from the map and
keeps its bytes as an unknown field of the parent, while protobuf C++ and
protobuf-python's pure backend both put the entry in the map. A map field is
shorthand for a repeated `MapEntry` message, and an unknown field inside a
submessage is skipped while the submessage still parses.

The corpus's job when two conformant runtimes read the same bytes differently is
to SAY SO, not to pick.
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = "armonik.ffi.corpus.v1"


def compile_protos(outdir):
    """protoc -> a FileDescriptorSet for each view, then message classes."""
    from google.protobuf import descriptor_pb2, descriptor_pool, message_factory
    pools = {}
    for view, fname in (("reader", "corpus.proto"), ("superset", "corpus_superset.proto")):
        desc = os.path.join(outdir, view + ".desc")
        r = subprocess.run([sys.executable, "-m", "grpc_tools.protoc", "-I", outdir,
                            "--descriptor_set_out=" + desc, fname], capture_output=True)
        if r.returncode:
            raise SystemExit("protoc failed on %s:\n%s" % (fname, r.stderr.decode()))
        fds = descriptor_pb2.FileDescriptorSet()
        with open(desc, "rb") as fh:
            fds.ParseFromString(fh.read())
        pool = descriptor_pool.DescriptorPool()
        for f in fds.file:
            pool.Add(f)
        pools[view] = (pool, message_factory)
        os.remove(desc)
    return pools


def msg_class(pools, view, name):
    pool, factory = pools[view]
    return factory.GetMessageClass(pool.FindMessageTypeByName("%s.%s" % (PKG, name)))


def runtime_id():
    import google.protobuf
    from google.protobuf.internal import api_implementation
    out = subprocess.run([sys.executable, "-m", "grpc_tools.protoc", "--version"],
                         capture_output=True)
    return {"runtime": "protobuf %s (%s backend)" % (google.protobuf.__version__,
                                                     api_implementation.Type()),
            "backend": api_implementation.Type(),
            "protoc": out.stdout.decode().strip()}


# ------------------------------------------------------------------ projection

def project(m):
    """What a reader must SEE, in a form every language can compare against.

    Not protobuf JSON: proto3 JSON omits a default value, which erases the
    difference between absent and present-and-zero, and that difference is the
    whole of vector class `empty`. This walks `ListFields`, whose semantics ("the
    fields that are set") are the same in every protobuf implementation -- which
    is exactly why a row where two runtimes' `ListFields` disagree is worth
    publishing rather than resolving.

    The encoding of each kind is fixed here and stated in CONTRACT.md section 3.
    """
    from google.protobuf.descriptor import FieldDescriptor as FD
    from google.protobuf.unknown_fields import UnknownFieldSet

    def val(fd, v):
        t = fd.type
        if t == FD.TYPE_MESSAGE:
            return project(v)
        if t == FD.TYPE_BYTES:
            return v.hex()
        if t == FD.TYPE_STRING:
            return v
        if t == FD.TYPE_BOOL:
            return bool(v)
        if t in (FD.TYPE_DOUBLE, FD.TYPE_FLOAT):
            return "%.17g" % v
        if t == FD.TYPE_ENUM or t in (FD.TYPE_INT32, FD.TYPE_INT64, FD.TYPE_UINT32,
                                      FD.TYPE_UINT64, FD.TYPE_SINT32, FD.TYPE_SINT64,
                                      FD.TYPE_FIXED32, FD.TYPE_FIXED64,
                                      FD.TYPE_SFIXED32, FD.TYPE_SFIXED64):
            return str(v)
        raise SystemExit("project() has no case for protobuf type %d (%s)" % (t, fd.name))

    out = {}
    for fd, v in m.ListFields():
        if fd.is_repeated and fd.message_type is not None and fd.message_type.GetOptions().map_entry:
            vfd = fd.message_type.fields_by_name["value"]
            out[fd.name] = dict((str(k), val(vfd, v[k])) for k in sorted(v))
        elif fd.is_repeated:
            out[fd.name] = [val(fd, x) for x in v]
        else:
            out[fd.name] = val(fd, v)
    u = unknown_list(UnknownFieldSet(m))
    if u:
        out["_unknown"] = u
    return out


def unknown_list(uset):
    out = []
    for f in uset:
        row = {"tag": f.field_number, "wire_type": f.wire_type}
        if f.wire_type == 3:
            row["group"] = unknown_list(f.data)
        elif f.wire_type == 2:
            row["hex"] = f.data.hex()
        else:
            row["value"] = str(f.data)
        out.append(row)
    return sorted(out, key=lambda r: (r["tag"], r["wire_type"]))


def unknown_tags_seen(m, acc=None):
    """Every unknown field number anywhere in the message, transitively."""
    from google.protobuf.unknown_fields import UnknownFieldSet
    from google.protobuf.descriptor import FieldDescriptor as FD
    acc = set() if acc is None else acc
    for f in UnknownFieldSet(m):
        acc.add(f.field_number)
    for fd, v in m.ListFields():
        if fd.type != FD.TYPE_MESSAGE:
            continue
        if fd.is_repeated and fd.message_type.GetOptions().map_entry:
            vfd = fd.message_type.fields_by_name["value"]
            if vfd.type == FD.TYPE_MESSAGE:
                for k in v:
                    unknown_tags_seen(v[k], acc)
        elif fd.is_repeated:
            for x in v:
                unknown_tags_seen(x, acc)
        else:
            unknown_tags_seen(v, acc)
    return acc


# ----------------------------------------------------------------- the reading

def read(pools, outdir, job):
    """One runtime's whole opinion of one vector."""
    row = {"id": job["id"]}
    # `path` is resolved by the caller, not re-derived from `file`. A baseline's
    # `file` is "../../schema/generated/payloads/P1_1.bin", relative to the
    # manifest's own directory, which only resolves when that directory sits two
    # levels deep -- and `--check` regenerates into a temporary directory at a
    # DIFFERENT path, which is the whole point of `--check`.
    path = job["path"]
    with open(path, "rb") as fh:
        data = fh.read()
    try:
        m = msg_class(pools, "reader", job["root"])()
        m.ParseFromString(data)
    except Exception as exc:                                   # noqa: BLE001
        row["parse"] = "error"
        row["error"] = "%s: %s" % (type(exc).__name__, str(exc)[:120])
        return row
    row["parse"] = "ok"
    row["projection"] = project(m)
    row["reencode"] = m.SerializeToString(deterministic=True).hex()
    row["unknown_tags"] = sorted(unknown_tags_seen(m))
    if job.get("superset"):
        try:
            sup = msg_class(pools, "superset", job.get("superset_root", job["root"]))()
            sup.ParseFromString(data)
            row["superset_projection"] = project(sup)
            row["superset_unknown_tags"] = sorted(unknown_tags_seen(sup))
        except Exception as exc:                               # noqa: BLE001
            row["superset_error"] = "%s: %s" % (type(exc).__name__, str(exc)[:120])
    return row


def main():
    outdir = sys.argv[1]
    jobs = json.load(sys.stdin)
    pools = compile_protos(outdir)
    out = {"runtime": runtime_id(), "rows": {}}
    for job in jobs:
        out["rows"][job["id"]] = read(pools, outdir, job)
    json.dump(out, sys.stdout)


if __name__ == "__main__":
    main()
