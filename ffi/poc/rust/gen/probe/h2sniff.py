#!/usr/bin/env python3
"""A Unix-socket forwarding proxy that prints the HTTP/2 connection setup of every connection
(the client preface, SETTINGS and WINDOW_UPDATE frames in both directions, until the first
HEADERS frame each way) and the frame-size pattern of the client's DATA frames (count,
max length). CONTAINER INSTRUMENTATION for the stream probe; never on a timed path.
   h2sniff.py LISTEN_SOCK UPSTREAM_SOCK LOG"""
import asyncio, struct, sys, os
LISTEN, UP, LOG = sys.argv[1], sys.argv[2], sys.argv[3]
NAMES = {1: "HEADER_TABLE_SIZE", 2: "ENABLE_PUSH", 3: "MAX_CONCURRENT_STREAMS", 4: "INITIAL_WINDOW_SIZE",
         5: "MAX_FRAME_SIZE", 6: "MAX_HEADER_LIST_SIZE", 8: "ENABLE_CONNECT_PROTOCOL"}
TYPES = {0: "DATA", 1: "HEADERS", 3: "RST_STREAM", 4: "SETTINGS", 6: "PING", 7: "GOAWAY", 8: "WINDOW_UPDATE"}
conn_no = 0
log = open(LOG, "a", buffering=1)

async def pump(r, w, tag, preface, stats):
    buf = b""
    need_preface = preface
    try:
        while True:
            d = await r.read(1 << 16)
            if not d:
                break
            w.write(d)
            await w.drain()
            buf += d
            if need_preface and len(buf) >= 24:
                buf = buf[24:]
                need_preface = False
            while not need_preface and len(buf) >= 9:
                ln = int.from_bytes(buf[:3], "big"); ty = buf[3]; fl = buf[4]
                sid = int.from_bytes(buf[5:9], "big") & 0x7FFFFFFF
                if len(buf) < 9 + ln:
                    break
                payload = buf[9:9 + ln]; buf = buf[9 + ln:]
                if ty == 4 and not (fl & 1):
                    s = {NAMES.get(struct.unpack(">H", payload[i:i + 2])[0], payload[i:i + 2].hex()): struct.unpack(">I", payload[i + 2:i + 6])[0]
                         for i in range(0, len(payload), 6)}
                    log.write(f"{tag} SETTINGS {s}\n")
                elif ty == 8 and stats["setup"]:
                    log.write(f"{tag} WINDOW_UPDATE stream {sid} +{int.from_bytes(payload[:4], 'big') & 0x7FFFFFFF}\n")
                elif ty == 1:
                    stats["setup"] = False
                elif ty == 0:
                    stats["data"] += 1; stats["max"] = max(stats["max"], ln); stats["bytes"] += ln
    finally:
        w.close()

async def handle(cr, cw):
    global conn_no
    conn_no += 1
    n = conn_no
    ur, uw = await asyncio.open_unix_connection(UP)
    c2s = {"setup": True, "data": 0, "max": 0, "bytes": 0}
    s2c = {"setup": True, "data": 0, "max": 0, "bytes": 0}
    log.write(f"== connection {n}\n")
    await asyncio.gather(pump(cr, uw, f"[{n}] client->server", True, c2s), pump(ur, cw, f"[{n}] server->client", False, s2c))
    log.write(f"[{n}] closed: client DATA frames {c2s['data']}, max length {c2s['max']}, bytes {c2s['bytes']}\n")

async def main():
    if os.path.exists(LISTEN):
        os.unlink(LISTEN)
    srv = await asyncio.start_unix_server(handle, LISTEN)
    async with srv:
        await srv.serve_forever()

asyncio.run(main())
