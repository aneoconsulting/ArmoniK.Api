#!/usr/bin/env python3
"""Kernel cycles of a `perf record -e cycles --call-graph lbr` profile split by NETWORK path, for the
UDS against TCP loopback attribution (owner, 2026-10-01).

  perf_net_attrib.py SYSTEM_MAP PERF.data PROFILE.out  -> one JSON object, ms per call (cycles / 3.3e9)

Kernel symbols from the booted kernel's System.map at the KASLR offset perf_attrib.kaslr finds.
A kernel sample's bucket is the first of these found walking its kernel call chain from the leaf
towards the syscall entry:
  netfilter         [module] (an address past _etext: nf_tables, nf_conntrack, nf_nat, br_netfilter,
                    xt_* are loaded here) and the core's nf_hook_slow, nf_* entry points
  wakeup            try_to_wake_up, __wake_up*, ep_poll_callback, sock_def_readable, *wake*
  receive softirq   net_rx_action, __netif_receive_skb*, process_backlog, ip_rcv*, ip_local_deliver*,
                    tcp_v4_rcv, tcp_v6_rcv, tcp_rcv_established, tcp_data_queue, tcp_ack, tcp_v4_do_rcv
  loopback transmit loopback_xmit, __dev_queue_xmit, dev_hard_start_xmit, ip_finish_output*, ip_output,
                    __ip_queue_xmit, ip6_*xmit, __tcp_transmit_skb, tcp_write_xmit, __tcp_push_pending_frames
  tcp send path     tcp_sendmsg*, skb allocation and copy under it
  unix send path    unix_stream_sendmsg
  socket read       tcp_recvmsg*, unix_stream_recvmsg, unix_stream_read_generic, sock_recvmsg
  epoll / futex / scheduling / page faults / other, as gen/perf_attrib.py
and separately: the cycles of kernel samples whose USER caller is a socket-write wrapper (writev,
sendmsg, sendto, write) AND whose chain holds a receive-softirq symbol ("receive under the write").
"""
import collections
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import perf_attrib as pa  # noqa: E402

GHZ = 3.3e9
RULES = [
    ("k: netfilter (nf_hook_slow and module code: conntrack, nat, nf_tables, br_netfilter)",
     re.compile(r"^(\[module\]|nf_hook_slow.*|nf_conntrack.*|nf_nat.*|nf_ct_.*|nft_.*|nf_tables.*|ipt_.*|ip6t_.*|xt_.*|br_nf_.*|__nf_.*)$")),
    ("k: wakeup (try_to_wake_up, ep_poll_callback, sock_def_readable)",
     re.compile(r"^(try_to_wake_up|__wake_up.*|ep_poll_callback|sock_def_readable|.*wake_up.*|ttwu_.*)$")),
    ("k: loopback receive (softirq: net_rx_action .. tcp_rcv_established)",
     re.compile(r"^(net_rx_action|__netif_receive_skb.*|process_backlog|ip_rcv.*|ip_local_deliver.*|tcp_v4_rcv|tcp_v6_rcv|"
                r"tcp_rcv_established|tcp_data_queue|tcp_ack|tcp_v4_do_rcv|tcp_v6_do_rcv|ipv6_rcv|ip6_rcv.*|ip6_input.*|"
                r"tcp_event_data_recv|tcp_queue_rcv|__tcp_ack_snd_check|tcp_rcv_space_adjust)$")),
    ("k: loopback transmit (tcp_write_xmit .. loopback_xmit)",
     re.compile(r"^(loopback_xmit|__dev_queue_xmit|dev_hard_start_xmit|ip_finish_output.*|ip_output|__ip_queue_xmit|"
                r"ip6_.*xmit|ip6_finish_output.*|ip6_output|__tcp_transmit_skb|tcp_write_xmit|__tcp_push_pending_frames|"
                r"tcp_push|__netif_rx|netif_rx.*|enqueue_to_backlog)$")),
    ("k: tcp send path (tcp_sendmsg: skb alloc, copy)", re.compile(r"^(tcp_sendmsg.*)$")),
    ("k: unix send path (unix_stream_sendmsg)", re.compile(r"^(unix_stream_sendmsg)$")),
    ("k: socket read", re.compile(r"^(tcp_recvmsg.*|unix_stream_recvmsg|unix_stream_read_generic|sock_recvmsg|tcp_cleanup_rbuf)$")),
]
RCV = RULES[2][1]


def main(smap, data, prof):
    pj = next((json.loads(l)["profile"] for l in open(prof) if l.startswith('{"profile"')), None)
    calls = pj["calls"] if pj else 1
    addrs, names = pa.load_map(smap)
    samples = pa.parse(data)
    off, share = pa.kaslr(samples, addrs, names)

    kname = pa.resolver(addrs, names, off)
    b = collections.Counter()
    under_write = 0
    kernel = user = 0
    rcv_anywhere = 0
    module_leaf = 0
    for s in samples:
        fr = s["frames"]
        if not fr:
            continue
        p = s["period"]
        if fr[0][0] < 0xffff800000000000:
            user += p
            b["u: user (all)"] += p
            continue
        kernel += p
        ks = [kname(f[0]) for f in fr if f[0] >= 0xffff800000000000]
        if ks and ks[0] == "[module]":
            module_leaf += p
        us = [f[1] for f in fr if f[0] < 0xffff800000000000]
        w = next((u for u in us if not re.search(r"syscall_cancel|__internal_syscall", u)), "")
        cat = None
        for k in ks:
            for name, rx in RULES:
                if rx.match(k):
                    cat = name
                    break
            if cat:
                break
        if cat is None:
            cat = pa.bucket(fr, kname)
            if cat in ("k: socket write (sendmsg/writev)", "k: socket read (recvmsg/read)"):
                cat = cat + " (other kernel under it)"
        b[cat] += p
        if any(RCV.match(k) for k in ks):
            rcv_anywhere += p
            if re.search(r"writev|sendmsg|sendto|(^|_)write$|__libc_write", w):
                under_write += p
    ms = lambda v: v / calls / GHZ * 1e3
    print(json.dumps({
        "profile": {k: pj[k] for k in ("cell", "payload", "dir", "k", "calls", "core_target")} if pj else None,
        "kaslr_offset": hex(off), "kaslr_check": round(share, 4), "samples": len(samples),
        "ms_per_call": {"total": ms(kernel + user), "kernel": ms(kernel), "user": ms(user),
                        "receive_softirq_anywhere": ms(rcv_anywhere), "receive_softirq_under_a_socket_write": ms(under_write),
                        "module_code_at_the_leaf": ms(module_leaf)},
        "kernel_buckets_ms_per_call": {k: ms(v) for k, v in sorted(b.items()) if k.startswith("k:")},
    }, sort_keys=True))


if __name__ == "__main__":
    main(*sys.argv[1:4])
