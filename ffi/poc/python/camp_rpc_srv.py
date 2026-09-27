"""The shared campaign server's launcher, from Python (FIX-PLAN WP10): poc/rust/serve.sh start,
warm and stop, and the socket list it prints. The server is the Rust slice's tonic rpc_server;
its interface is poc/rust/SERVER.md. Imports nothing heavy (the pyperf master process uses it)."""
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))


def serve_sh():
    """poc/rust/serve.sh, the shared server's launcher: AK_SERVE_SH, else the runner's snapshot
    (AK_SNAPSHOT_DIR, the commit the run names), else this checkout's."""
    if os.environ.get("AK_SERVE_SH"):
        return os.environ["AK_SERVE_SH"]
    snap = os.environ.get("AK_SNAPSHOT_DIR")
    if snap and os.path.exists(os.path.join(snap, "ffi", "poc", "rust", "serve.sh")):
        return os.path.join(snap, "ffi", "poc", "rust", "serve.sh")
    return os.path.normpath(os.path.join(HERE, "..", "rust", "serve.sh"))


class Server:
    """This process's own start of the shared server (serve.sh start / warm / stop), for a run
    with no --server: the gate's must-fail controls and rpc_counts.py. run_campaign.sh starts
    ONE per launch instead (req 13 as amended). Its state file is private to this start."""

    def __init__(self, d, warm=0):
        import tempfile
        self.env = dict(os.environ, AK_SERVE_STATE=os.path.join(tempfile.mkdtemp(prefix="akserve"), "state"))
        r = subprocess.run([serve_sh(), "start", "--out", d], env=self.env, capture_output=True, text=True)
        if r.returncode:
            raise RuntimeError("serve.sh start failed: %s" % (r.stderr or r.stdout).strip()[-300:])
        self.info = parse_server(r.stdout)
        if warm:
            w = subprocess.run([serve_sh(), "warm", str(warm)], env=self.env, capture_output=True, text=True)
            if w.returncode:
                self.stop()
                raise RuntimeError("serve.sh warm failed: %s" % (w.stderr or w.stdout).strip()[-300:])

    def stop(self):
        subprocess.run([serve_sh(), "stop"], env=self.env, capture_output=True, text=True)


def parse_server(text):
    """serve.sh start's `shipped PATH` / `pinned PATH` / `pid N` lines, or the runner's
    `shipped=unix:PATH,pinned=unix:PATH` -> {"shipped": "unix:PATH", "pinned": "unix:PATH", ...}."""
    out = {}
    for x in text.replace(",", "\n").splitlines():
        x = x.strip()
        if "=" in x:
            k, v = x.split("=", 1)
        elif " " in x:
            k, v = x.split(None, 1)
            if k in ("shipped", "pinned"):
                v = "unix:" + v
        else:
            continue
        out[k] = v
    if "shipped" not in out or "pinned" not in out:
        raise RuntimeError("no server sockets in %r" % text[:200])
    return out
