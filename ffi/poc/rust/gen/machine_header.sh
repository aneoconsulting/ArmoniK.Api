# Sourced by the session drivers: `machine_header` prints the machine and isolation lines of a log
# header, read at run time from sysfs, procfs and the cgroup tree (2026-09-30: the owner confines
# non-benchmark work to the OS set with systemd AllowedCPUs, IRQ affinity and taskset).
machine_header() {
  local sysf; sysf() { cat "$1" 2>/dev/null || echo n/a; }
  echo "# cpu        $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'); kernel $(uname -r); smt $(sysf /sys/devices/system/cpu/smt/control) (active $(sysf /sys/devices/system/cpu/smt/active)); no_turbo $(sysf /sys/devices/system/cpu/intel_pstate/no_turbo); governor cpu1 $(sysf /sys/devices/system/cpu/cpu1/cpufreq/scaling_governor); scaling min/max cpu1 $(sysf /sys/devices/system/cpu/cpu1/cpufreq/scaling_min_freq)/$(sysf /sys/devices/system/cpu/cpu1/cpufreq/scaling_max_freq) kHz"
  echo "# isolation  cmdline: $(tr ' ' '\n' < /proc/cmdline | grep -E '^(isolcpus|nohz_full|rcu_nocbs|irqaffinity)=' | tr '\n' ' ')isolated='$(sysf /sys/devices/system/cpu/isolated)' nohz_full='$(sysf /sys/devices/system/cpu/nohz_full)'"
  local s line=""
  for s in init.scope system.slice user.slice machine.slice; do
    [ -d "/sys/fs/cgroup/$s" ] && line="$line $s=$(sysf /sys/fs/cgroup/$s/cpuset.cpus.effective)"
  done
  local cg; cg=$(sed -n 's/^0:://p' /proc/self/cgroup)
  echo "# cgroups    cpuset.cpus.effective:$line; this driver's cgroup $cg: $(sysf /sys/fs/cgroup$cg/cpuset.cpus.effective); root cpuset.cpus.isolated='$(sysf /sys/fs/cgroup/cpuset.cpus.isolated)'; this driver's affinity $(taskset -pc $$ | sed 's/.*: //')"
  echo "# irq        default_smp_affinity $(sysf /proc/irq/default_smp_affinity); smp_affinity_list of /proc/irq/*: $(cat /proc/irq/*/smp_affinity_list 2>/dev/null | sort | uniq -c | awk '{printf "%s x%s, ", $2, $1}')"
  echo "# allowed    non-kernel threads whose Cpus_allowed_list includes a benchmark CPU (1-8, 11-18), this session's excluded: $(python3 -S - <<'PY'
import os, re, collections
BENCH = set(range(1, 9)) | set(range(11, 19))
MINE = re.compile(r"stream_probe|rpc_server|rpc_warm|caller|cell-rt|tokio-rt|ak-reactor|taskset|flock|bash|python3|cargo|perf|strace")
def cpus(s):
    out = set()
    for part in s.split(","):
        a, _, b = part.partition("-")
        if a:
            out.update(range(int(a), int(b or a) + 1))
    return out
seen = collections.Counter()
for pid in os.listdir("/proc"):
    if not pid.isdigit():
        continue
    try:
        if not open(f"/proc/{pid}/cmdline", "rb").read():
            continue  # a kernel thread
        for tid in os.listdir(f"/proc/{pid}/task"):
            st = open(f"/proc/{pid}/task/{tid}/status").read()
            name = re.search(r"^Name:\s*(.*)$", st, re.M).group(1)
            allowed = cpus(re.search(r"^Cpus_allowed_list:\s*(.*)$", st, re.M).group(1))
            if allowed & BENCH and not MINE.search(name):
                seen[name] += 1
    except (OSError, AttributeError):
        pass
print(", ".join(f"{n} x{c}" for n, c in seen.most_common(20)) or "none")
PY
)"
  echo "# running    threads RUNNABLE right now on the client or server CPUs (1-8, 11-18), this session's excluded: $(ps -eLo psr=,stat=,comm= | awk '(($1>=1&&$1<=8)||($1>=11&&$1<=18)) && $2 ~ /^R/ {print $3}' | grep -vE 'stream_probe|rpc_server|caller|cell-rt|tokio-rt|ak-reactor|^ps$' | sort | uniq -c | sort -rn | head -12 | awk '{printf "%s x%s, ", $2, $1}'); loadavg $(cat /proc/loadavg)"
}
