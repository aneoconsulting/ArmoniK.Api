# Sourced by the timed drivers: the endpoint every client dials (owner, 2026-10-01: TCP only from now on).
#
#   AK_NET=tcp (default)  the shared server's 127.0.0.1 listener (poc/rust/serve.sh with AK_SERVER_TCP=0:
#                         the pinned configuration, TCP_NODELAY set on accept); grpc++ dials ipv4:127.0.0.1:PORT,
#                         the core http://127.0.0.1:PORT (its client passes tcp_nagle 0)
#   AK_NET=uds            the pinned Unix socket, as every run before 2026-10-01
#
#   net_server_env              the environment `serve.sh start` needs (AK_SERVER_TCP=0 on TCP)
#   net_endpoints STATE_FILE    sets NET_TGT (grpc++ --target), NET_CTGT (--core-target), NET_DESC, NET_PORT;
#                               refuses (return 1) on TCP when the server has no tcp line
#   net_nodelay_ok FILE         on TCP: 0 when the client's output reports tcp_sockets_n > 0 and every one of
#                               them read TCP_NODELAY = 1 (getsockopt on the live socket, campaign_rpc's
#                               tcp_sockets in the profile JSON and in the grid header); always 0 on UDS
AK_NET=${AK_NET:-tcp}
case "$AK_NET" in tcp|uds) ;; *) echo "AK_NET must be tcp or uds, not '$AK_NET'" >&2; return 2 2>/dev/null || exit 2 ;; esac

net_server_env() { [ "$AK_NET" = tcp ] && echo "AK_SERVER_TCP=0"; }

net_endpoints() {
  local st=$1 sock tcpa
  sock=$(sed -n 's/^pinned //p' "$st"); tcpa=$(sed -n 's/^tcp //p' "$st")
  if [ "$AK_NET" = tcp ]; then
    [ -n "$tcpa" ] || { echo "REFUSED: AK_NET=tcp but the server state $st has no tcp line (start it with AK_SERVER_TCP=0)"; return 1; }
    NET_TGT="ipv4:$tcpa"; NET_CTGT="http://$tcpa"; NET_PORT=${tcpa##*:}
    NET_DESC="tcp: grpc++ ipv4:$tcpa, core http://$tcpa (tcp_nagle 0; server TCP_NODELAY on accept); TCP_NODELAY read back on every client socket of every process"
  else
    NET_TGT="unix:$sock"; NET_CTGT="unix:$sock"; NET_PORT=""
    NET_DESC="uds: unix:$sock (pinned)"
  fi
}

net_nodelay_ok() {
  [ "$AK_NET" = tcp ] || return 0
  python3 -c "import re,sys
m=re.findall(r'\"tcp_nodelay_on\": (\d+), \"tcp_sockets_n\": (\d+)', open(sys.argv[1], errors='replace').read())
sys.exit(0 if m and all(int(a)==int(b)>0 for a,b in m) else 1)" "$1"
}
