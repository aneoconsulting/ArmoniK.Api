#!/usr/bin/env python3
"""The machine facts a physical-machine log header records, read from sysfs and /proc at run
time (nothing hard-coded): CPU model, kernel, SMT, governor, scaling min/max/current
frequency, no_turbo / boost, the isolation mechanism (isolated, nohz_full, the kernel command
line, the cgroup cpuset and its partition type), the CPU sets and their SMT siblings, the load
and the busiest other processes, and, for a running server, its pid, affinity, thread count and
AK_SERVER_THREADS.

  machine_facts.py CLIENT_SET SERVER_SET [SERVER_PID ...]   -> one JSON object on stdout
"""
import collections
import glob
import json
import os
import subprocess
import sys


def rd(p):
    try:
        with open(p) as f:
            return f.read().strip()
    except OSError:
        return None


def rng(s):
    out = set()
    for part in (s or "").split(","):
        part = part.strip()
        if "-" in part:
            a, b = part.split("-")
            out.update(range(int(a), int(b) + 1))
        elif part:
            out.add(int(part))
    return sorted(out)


def per_cpu(cpus, name):
    vals = {c: rd("/sys/devices/system/cpu/cpu%d/cpufreq/%s" % (c, name)) for c in cpus}
    distinct = sorted(set(v for v in vals.values() if v is not None))
    return distinct[0] if len(distinct) == 1 else vals


def proc_status(pid, key):
    s = rd("/proc/%s/status" % pid) or ""
    for line in s.splitlines():
        if line.startswith(key + ":"):
            return line.split(":", 1)[1].strip()
    return None


def server(pid):
    env = {}
    try:
        with open("/proc/%s/environ" % pid, "rb") as f:
            for kv in f.read().split(b"\0"):
                if kv.startswith(b"AK_"):
                    k, _, v = kv.partition(b"=")
                    env[k.decode()] = v.decode()
    except OSError:
        pass
    comms = {}
    for t in glob.glob("/proc/%s/task/*/comm" % pid):
        c = rd(t)
        comms[c] = comms.get(c, 0) + 1
    return {"pid": int(pid), "alive": os.path.exists("/proc/%s" % pid),
            "cmdline": (rd("/proc/%s/cmdline" % pid) or "").replace("\0", " ").strip(),
            "cpus_allowed_list": proc_status(pid, "Cpus_allowed_list"),
            "threads": proc_status(pid, "Threads"), "thread_classes": comms, "ak_env": env}


def confinement(measured):
    """How non-benchmark work is kept off the measured CPUs, as the system reports it: the cpuset of
    the top-level cgroups (systemd AllowedCPUs lands in cpuset.cpus), the IRQs whose affinity
    reaches a measured CPU, and the threads outside this process tree allowed on one."""
    m = set(measured)
    slices = {}
    for d in sorted(glob.glob("/sys/fs/cgroup/*.slice") + glob.glob("/sys/fs/cgroup/*.scope")) + ["/sys/fs/cgroup"]:
        slices[d.replace("/sys/fs/cgroup", "") or "/"] = {"cpus": rd(d + "/cpuset.cpus"), "effective": rd(d + "/cpuset.cpus.effective"),
                                                          "partition": rd(d + "/cpuset.cpus.partition")}
    irq_total, irq_on = 0, []
    for f in glob.glob("/proc/irq/*/smp_affinity_list"):
        v = rd(f)
        if v is None:
            continue
        irq_total += 1
        eff = rd(f.replace("smp_affinity_list", "effective_affinity_list"))
        if set(rng(v)) & m:
            irq_on.append("%s(eff %s)" % (f.split("/")[3], eff))
    me = os.getpid()
    anc = set()
    p = me
    while p > 1:
        anc.add(p)
        st = rd("/proc/%d/stat" % p)
        try:
            p = int(st.rsplit(")", 1)[1].split()[1])
        except (AttributeError, IndexError, ValueError):
            break
    allowed_on = collections.Counter()
    user_on = []
    nthreads = 0
    for t in glob.glob("/proc/[0-9]*/task/[0-9]*/status"):
        pid = int(t.split("/")[2])
        if pid in anc:
            continue
        s = rd(t) or ""
        cpus = next((l.split(":", 1)[1].strip() for l in s.splitlines() if l.startswith("Cpus_allowed_list")), None)
        if cpus is None:
            continue
        nthreads += 1
        if set(rng(cpus)) & m:
            comm = rd("/proc/%d/comm" % pid) or "?"
            allowed_on[comm] += 1
            # a kernel thread has PF_KTHREAD (0x00200000) in its stat flags (field 9)
            try:
                flags = int((rd("/proc/%d/stat" % pid) or "").rsplit(")", 1)[1].split()[6])
            except (IndexError, ValueError):
                flags = 0
            if not flags & 0x00200000:
                tcomm = rd(t.replace("/status", "/comm")) or "?"
                user_on.append("%s[%d]/%s:%s" % (comm, pid, tcomm, cpus))
    return {"cgroup_cpusets": slices, "irqs": irq_total, "irqs_reaching_measured_cpus": len(irq_on),
            "irqs_reaching_measured_cpus_list": irq_on[:40], "threads_seen": nthreads,
            "threads_allowed_on_measured_cpus": sum(allowed_on.values()),
            "threads_allowed_on_measured_cpus_by_comm": dict(allowed_on.most_common(25)),
            "non_kernel_threads_allowed_on_measured_cpus": user_on[:60], "non_kernel_count": len(user_on),
            "note": "threads of this process's ancestors are left out; kernel per-CPU threads (kworker/N, ksoftirqd/N, migration/N) are bound by design"}


def main():
    client, srv = rng(sys.argv[1]), rng(sys.argv[2])
    cmd = rd("/proc/cmdline") or ""
    cg = (rd("/proc/self/cgroup") or "").split("\n")[0].split(":")[-1]
    cgdir = "/sys/fs/cgroup" + cg
    sib = {c: rd("/sys/devices/system/cpu/cpu%d/topology/thread_siblings_list" % c) for c in client + srv}
    busy = subprocess.run(["ps", "-eo", "pcpu,psr,comm", "--sort=-pcpu", "--no-headers"],
                          stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True).stdout.splitlines()[:8]
    out = {
        "cpu_model": next((l.split(":", 1)[1].strip() for l in (rd("/proc/cpuinfo") or "").splitlines()
                           if l.startswith("model name")), None),
        "kernel": os.uname().release,
        "cpus_online": rd("/sys/devices/system/cpu/online"),
        "smt": {"active": rd("/sys/devices/system/cpu/smt/active"), "control": rd("/sys/devices/system/cpu/smt/control")},
        "governor": per_cpu(client + srv, "scaling_governor"),
        "scaling_driver": per_cpu(client + srv, "scaling_driver"),
        "scaling_min_freq_khz": per_cpu(client + srv, "scaling_min_freq"),
        "scaling_max_freq_khz": per_cpu(client + srv, "scaling_max_freq"),
        "scaling_cur_freq_khz": {c: rd("/sys/devices/system/cpu/cpu%d/cpufreq/scaling_cur_freq" % c) for c in client + srv},
        "no_turbo": rd("/sys/devices/system/cpu/intel_pstate/no_turbo"),
        "boost": rd("/sys/devices/system/cpu/cpufreq/boost"),
        "isolation": {
            "sys_isolated": rd("/sys/devices/system/cpu/isolated"),
            "sys_nohz_full": rd("/sys/devices/system/cpu/nohz_full"),
            "cmdline": [t for t in cmd.split() if t.split("=")[0] in
                        ("isolcpus", "nohz_full", "rcu_nocbs", "irqaffinity", "nohz", "intel_pstate", "processor.max_cstate", "idle")],
            "cgroup": cg, "cgroup_cpuset_effective": rd(cgdir + "/cpuset.cpus.effective"),
            "cgroup_cpuset_partition": rd(cgdir + "/cpuset.cpus.partition"),
            "mechanism": ("isolcpus/nohz_full" if (rd("/sys/devices/system/cpu/isolated") or rd("/sys/devices/system/cpu/nohz_full"))
                          else "cpuset partition " + rd(cgdir + "/cpuset.cpus.partition")
                          if (rd(cgdir + "/cpuset.cpus.partition") or "member") != "member"
                          else "none: taskset affinity only (sys isolated and nohz_full empty, no cpuset partition on this cgroup)"),
        },
        "cpu_sets": {"CLIENT": sys.argv[1], "SERVER": sys.argv[2],
                     "OS": ",".join(str(c) for c in rng(rd("/sys/devices/system/cpu/online")) if c not in client + srv),
                     "siblings": sib},
        "loadavg": rd("/proc/loadavg"),
        "confinement": confinement(client + srv),
        "busiest_processes_pcpu_psr_comm": busy,
        "servers": [server(p) for p in sys.argv[3:]],
    }
    c = out["confinement"]
    conf = ["%s=%s" % (k, v["cpus"]) for k, v in sorted(c["cgroup_cpusets"].items()) if v.get("cpus")]
    if conf and not (rd("/sys/devices/system/cpu/isolated") or rd("/sys/devices/system/cpu/nohz_full")):
        out["isolation"]["mechanism"] = ("cgroup cpusets (systemd AllowedCPUs) on %s; unconfined cgroups: %s; IRQs reaching the "
                                         "measured CPUs: %d of %d; threads outside this process tree allowed on them: %d (non-kernel: %d)"
                                         % (", ".join(conf), ", ".join(k for k, v in sorted(c["cgroup_cpusets"].items()) if not v.get("cpus")) or "none",
                                            c["irqs_reaching_measured_cpus"], c["irqs"], c["threads_allowed_on_measured_cpus"], c["non_kernel_count"]))
    print(json.dumps(out, sort_keys=True))


if __name__ == "__main__":
    main()
