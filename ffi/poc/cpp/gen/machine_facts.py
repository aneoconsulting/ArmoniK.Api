#!/usr/bin/env python3
"""The machine facts a physical-machine log header records, read from sysfs and /proc at run
time (nothing hard-coded): CPU model, kernel, SMT, governor, scaling min/max/current
frequency, no_turbo / boost, the isolation mechanism (isolated, nohz_full, the kernel command
line, the cgroup cpuset and its partition type), the CPU sets and their SMT siblings, the load
and the busiest other processes, and, for a running server, its pid, affinity, thread count and
AK_SERVER_THREADS.

  machine_facts.py CLIENT_SET SERVER_SET [SERVER_PID ...]   -> one JSON object on stdout
"""
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
        "busiest_processes_pcpu_psr_comm": busy,
        "servers": [server(p) for p in sys.argv[3:]],
    }
    print(json.dumps(out, sort_keys=True))


if __name__ == "__main__":
    main()
