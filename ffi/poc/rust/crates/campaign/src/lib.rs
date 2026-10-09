//! FIX-PLAN WP3: the rust slice's campaign harness (design/CAMPAIGN.md).
//!
//! What lives here: the per-root arm table (`Ops`, rendered by `gen/rust_campaign.py`), the
//! case list of the codec suite (payloads, content sets, `U-*` rows; arms; directions;
//! unknown-field modes; the encode variants of requirement 11), the process-CPU measurement
//! criterion runs with (requirement 21 as amended), the RPC grid's cells and server (`grid`,
//! `server`), the JSON-lines
//! writer of section 7, and the header of every log (requirement 27).
//!
//! Arms (requirement 8), as rows:
//!   incumbent-prod  prost through the calls tonic's codec makes (`tonic_prost::ProstEncoder`
//!                   is `item.encode(&mut EncodeBuf)` over a `BytesMut`; `ProstDecoder` is
//!                   `Message::decode(&mut DecodeBuf)` over a `Buf`): encode into a reused
//!                   `BytesMut`, decode from a `bytes::Bytes`. `incumbent-best` is the SAME
//!                   entry point (`Message::encode`/`decode`, R14), so it is not a second row.
//!   armonik         the facade types with their generated `prost::Message` impls
//!   core-native     the codec generated into the host (Rust's `host-gen`), drop and retain
//!   core-ffi        the generated binding through the C ABI, drop and retain; decode with the
//!                   FSM family (FIX-PLAN D24: ak_fsm_begin_<R> + ak_fsm_next_<R>, each event
//!                   fed to the binding's host functions as it arrives, binding fsm_with_<root>)
//!   core-ffi-push   labelled extra (D24): the push family (ak_decode_<R> + vtable reverse
//!                   calls, binding decode_with_<root>), decode only, drop and retain
//!   core-ffi-pull   labelled extra: the pull family (walk in place), decode only, drop and retain

pub mod generated {
    pub mod roots;
}
pub mod grid;
pub mod server;

use criterion::measurement::{Measurement, ValueFormatter};
use criterion::Throughput;
use harness::arms::core_ffi_arm::Ctx;
use std::path::PathBuf;

/// Optimisation T1 (ffi): an owned buffer the core handed over (`ak_enc_take_owned`), held
/// as a `Bytes` owner. Dropping the last `Bytes` clone releases it (`ak_bytes_free`, which
/// the core makes safe on any thread: the transport drops a request body where it likes).
struct AkOwned(ak_abi::ak_bytes);
// SAFETY: the bytes are immutable while owned, and `ak_bytes_free` may run on any thread.
unsafe impl Send for AkOwned {}
unsafe impl Sync for AkOwned {}
impl AsRef<[u8]> for AkOwned {
    fn as_ref(&self) -> &[u8] {
        if self.0.len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(self.0.ptr, self.0.len) } }
    }
}
impl Drop for AkOwned {
    fn drop(&mut self) {
        unsafe { ak_abi::ak_bytes_free(&mut self.0) }
    }
}

/// T1 (ffi): cell D's request body -- the core-ffi encode context's output MOVED to the
/// host (`ak_enc_take_owned`, one crossing) and wrapped as a `Bytes` without a copy
/// (`Bytes::from_owner`); the release is the second crossing, when the body is dropped.
/// An owned `ak_bytes` (released with ak_bytes_free when the last clone drops) as a `Bytes`.
pub fn owned_bytes(b: ak_abi::ak_bytes) -> bytes::Bytes {
    bytes::Bytes::from_owner(AkOwned(b))
}

pub fn ffi_owned_body(enc: *mut ak_abi::ak_enc_ctx) -> Result<bytes::Bytes, i32> {
    let mut b = ak_abi::ak_bytes { ptr: std::ptr::null(), len: 0, owner: std::ptr::null_mut() };
    let rc = unsafe { ak_abi::ak_enc_take_owned(enc, &mut b) };
    if rc != ak_abi::AK_OK {
        return Err(rc);
    }
    Ok(bytes::Bytes::from_owner(AkOwned(b)))
}

/// Requirement 21: CPU time of the measuring thread, `CLOCK_THREAD_CPUTIME_ID`, in ns.
#[inline]
pub fn thread_cpu_ns() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

/// Requirement 21 for the RPC client: the process's user + system CPU (getrusage SELF), ns.
pub fn process_cpu_ns() -> u64 {
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    let tv = |t: libc::timeval| t.tv_sec as u64 * 1_000_000_000 + t.tv_usec as u64 * 1000;
    tv(ru.ru_utime) + tv(ru.ru_stime)
}

/// Requirement 21 as amended 2026-09-26 (R-H25): the codec suite's CPU is PROCESS CPU,
/// `CLOCK_PROCESS_CPUTIME_ID`, so any helper thread counts for every arm. In ns.
#[inline]
pub fn process_clock_ns() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts) };
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

/// CAMPAIGN req 25 as amended 2026-10-03 (D9): the allocator mode of a measured process.
/// `AK_CAMPAIGN_ALLOC=default` (the main figures: glibc's default allocator, no
/// GLIBC_TUNABLES) or `pinned` (the labelled diagnostic: PINNED_TUNABLES).
pub const PINNED_TUNABLES: &str = "glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432";

/// The minor page faults of this process so far (getrusage), read OUTSIDE every timer.
pub fn minflt() -> u64 {
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    ru.ru_minflt as u64
}

/// D9's startup check, before any timing: the mode asked for, the environment that should
/// produce it, and what glibc actually does with one 16 MiB malloc (mallinfo2's mmapped-block
/// count before and after: `mmapped` under the default 128 KiB..32 MiB dynamic threshold,
/// `heap` under the pinned 32 MiB threshold). Any disagreement REFUSES the run: exit 4, no
/// sample. Returns the mode and the readback for the header. Probed ONCE per process and the
/// block is KEPT (owner, after java 2892e207b): freeing a mmapped block raises glibc's
/// dynamic mmap threshold, so a freed probe would change the mode it verifies.
pub fn alloc_check() -> (&'static str, &'static str) {
    let mode: &'static str = match std::env::var("AK_CAMPAIGN_ALLOC").ok().as_deref() {
        None | Some("") | Some("default") => "default",
        Some("pinned") => "pinned",
        Some(o) => refuse(format!("AK_CAMPAIGN_ALLOC={o} (want default or pinned)")),
    };
    let tun = std::env::var("GLIBC_TUNABLES").unwrap_or_default();
    match mode {
        "default" if !tun.is_empty() => refuse(format!("AK_CAMPAIGN_ALLOC=default but GLIBC_TUNABLES={tun}")),
        "pinned" if tun != PINNED_TUNABLES => refuse(format!("AK_CAMPAIGN_ALLOC=pinned but GLIBC_TUNABLES={tun:?}, want {PINNED_TUNABLES}")),
        _ => {}
    }
    let read: &'static str = unsafe {
        let before = libc::mallinfo2().hblks;
        let p = libc::malloc(16 << 20);
        assert!(!p.is_null(), "16 MiB malloc");
        std::ptr::write_volatile(p as *mut u8, 1);
        let after = libc::mallinfo2().hblks;
        // never freed: see above
        PROBE.store(p as *mut u8, std::sync::atomic::Ordering::Relaxed);
        if after > before { "mmapped" } else { "heap" }
    };
    let want = if mode == "default" { "mmapped" } else { "heap" };
    if read != want {
        refuse(format!("AK_CAMPAIGN_ALLOC={mode}: a 16 MiB malloc was {read}, want {want}"));
    }
    eprintln!("# alloc: {mode} (16 MiB malloc {read})");
    (mode, read)
}

static PROBE: std::sync::atomic::AtomicPtr<u8> = std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

fn refuse(why: String) -> ! {
    eprintln!("REFUSED (CAMPAIGN req 25, D9): {why}; no sample is taken");
    std::process::exit(4);
}

// ---- CAMPAIGN section 4.0 (D18, owner 2026-10-03): the campaign grid -----------------------

/// `AK_CAMPAIGN_GRID=core|full` (default `core`): `core` runs exactly section 4.0's grid,
/// `full` runs every row of 4.1 and 4.2, each row labelled `core` or `extra`.
pub fn campaign_grid() -> &'static str {
    match std::env::var("AK_CAMPAIGN_GRID").ok().as_deref() {
        None | Some("") | Some("core") => "core",
        Some("full") => "full",
        Some(o) => refuse_grid(o),
    }
}

fn refuse_grid(o: &str) -> ! {
    eprintln!("REFUSED: AK_CAMPAIGN_GRID={o} (want core or full)");
    std::process::exit(2);
}

/// Section 4.0's seven timed `U-*` rows, one per ABI root.
pub const CORE_U_ROWS: [&str; 7] = [
    "U-nested-before", "U-deep-u-repeated", "U-oneof-u-repeated",
    "U-wire-ListTaskSummaryResponse-tasks-as-wt5", "U-wire-UploadResultDataMessage-upload-as-wt5",
    "U-wire-ListMetricsResponse-batches-as-wt0", "U-wire-DualResponse-left-as-wt5",
];

/// A codec input of the core grid: the 16 shapes (ASCII), P2.2's Latin-1 and wide sets, the
/// seven `U-*` rows.
pub fn core_codec_input(id: &str) -> bool {
    if id.starts_with("U-") {
        return CORE_U_ROWS.contains(&id);
    }
    !id.contains('/') || id == "P2.2/latin1" || id == "P2.2/wide"
}

/// A codec case of the core grid. Arms: incumbent-prod (full build only), core-ffi (FSM decode, D24)
/// and host-gen, which in Rust is `core-native` (the codec the shared generator writes into
/// Rust, no C ABI boundary). Encode: end state (ii), the form the arm's RPC path of the
/// core grid hands its transport (incumbent: tonic's `Bytes`, cell A; core-ffi and
/// core-native: the core transport's form, cells Cf and Ef), hot input. Decode: decode-read.
/// Modes: retain in the full build, no-unknown in the no-unknown build.
pub fn core_codec_case(arm: &str, dir: &str, mode: &str, end_state: &str, input: &str) -> bool {
    let full = cfg!(feature = "unknown-fields");
    let mode_ok = match arm {
        "incumbent-prod" => full,
        "core-ffi" | "core-native" => mode == if full { "retain" } else { "no-unknown" },
        _ => false,
    };
    let want_end = if arm == "incumbent-prod" { "transport-ready-tonic" } else { "transport-ready-core" };
    mode_ok && (dir == "decode-read" || (dir == "encode" && end_state == want_end && input == "hot"))
}

/// The codec extras the core grid leaves out (header).
pub const CODEC_EXTRAS: &str = "incumbent-best (none in Rust); core-ffi-push (push decode, D24); core-ffi-pull (pull decode); armonik; bare decode; the other encode variants (reused-buffer hot and pool, transport-ready pool, core-ffi's transport-ready-tonic and core-native's transport-ready-tonic); the drop mode; incumbent-prod and armonik in the no-unknown build; Latin-1 and wide on P1.2 and P2.4; the other U-* rows timed (all stay in the gate and in this process's pre-check of the timed inputs)";

/// An RPC benchmark of the core grid. The core cells are the framed ones with Rust's
/// idiomatic delivery, the callback bridged to async (req 16 as amended): Bf-cb, Cf-cb, Ef-cb.
///   main grid (default allocator, stock h2): A, Bf-cb, Cf-cb-retain, Ef-cb-retain;
///     a+read and b (P2.2), c at P5.4, d at 16 MiB; k = 1 and 8; full build
///   pinned allocator pass: A and Cf-cb-retain, c and d, k = 1
///   h2-batch: Cf-cb-retain, c and d, k = 1 and 8
pub fn core_rpc_spec(cell: &str, dir: &str, payload: &str, k: usize, alloc: &str, h2: &str) -> bool {
    if !cfg!(feature = "unknown-fields") {
        return false;
    }
    let pay = match dir {
        "a+read" | "b" => payload == "P2.2",
        "c" => payload == "P5.4",
        "d" => payload == "16MiB",
        _ => false,
    };
    let upload = dir == "c" || dir == "d";
    pay && if h2 == "h2-batch" {
        cell == "Cf-cb-retain" && upload && (k == 1 || k == 8)
    } else if alloc == "pinned" {
        (cell == "A" || cell == "Cf-cb-retain") && upload && k == 1
    } else {
        matches!(cell, "A" | "Bf-cb" | "Cf-cb-retain" | "Ef-cb-retain") && (k == 1 || k == 8)
    }
}

/// The RPC extras the core grid leaves out (header).
pub const RPC_EXTRAS: &str = "cells B, C, D, E, F, Df, Ff, the reference (non-framed) rows, the blocking delivery of Bf/Cf/Ef and the reference -cb rows; the drop mode; direction a; k = 16; P5.3; d at 4 MiB; the RPC grid in the no-unknown build; the second transport configuration (pinned windows); h2-batch on other rows; the pinned allocator pass beyond A and Cf-cb on c and d at k = 1";

/// The h2 variant of the loaded core (`AK_H2=stock|h2-batch`, default stock), checked
/// against the libak_core.so this process mapped: the h2-batch build compiles h2 from
/// `h2-batch-src` (its panic locations carry the path). Refuses on a mismatch. Returns
/// (variant, the mapped path).
pub fn h2_check() -> (&'static str, String) {
    let want: &'static str = match std::env::var("AK_H2").ok().as_deref() {
        None | Some("") | Some("stock") => "stock",
        Some("h2-batch") => "h2-batch",
        Some(o) => { eprintln!("REFUSED: AK_H2={o} (want stock or h2-batch)"); std::process::exit(5) }
    };
    let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
    let so = maps.lines().filter_map(|l| l.split_whitespace().nth(5)).find(|p| p.ends_with("/libak_core.so"))
        .map(String::from).unwrap_or_else(|| { eprintln!("REFUSED: no libak_core.so mapped"); std::process::exit(5) });
    let bytes = std::fs::read(&so).unwrap_or_default();
    let needle = b"h2-batch-src/src/codec/framed_write.rs";
    let got = if bytes.windows(needle.len()).any(|w| w == needle) { "h2-batch" } else { "stock" };
    if got != want {
        eprintln!("REFUSED: AK_H2={want} but the mapped core {so} is {got}");
        std::process::exit(5);
    }
    (want, so)
}

/// CAMPAIGN 4.0 as amended (b58543f7b): Nagle off on every client socket, read back on the live
/// sockets of THIS process. Every open fd that is a TCP socket connected to `peer` (host:port)
/// is asked `getsockopt(IPPROTO_TCP, TCP_NODELAY)`; any 0 REFUSES the run (exit 7). Returns the
/// number of sockets read back (0 also refuses: the transport is TCP, so a timed cell has one).
pub fn nodelay_readback(peer: &str) -> usize {
    let mut n = 0usize;
    let mut bad = Vec::new();
    for e in std::fs::read_dir("/proc/self/fd").into_iter().flatten().flatten() {
        let Ok(fd) = e.file_name().to_string_lossy().parse::<i32>() else { continue };
        let Ok(l) = std::fs::read_link(e.path()) else { continue };
        if !l.to_string_lossy().starts_with("socket:") {
            continue;
        }
        unsafe {
            let mut sa: libc::sockaddr_storage = std::mem::zeroed();
            let mut sl = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            if libc::getpeername(fd, &mut sa as *mut _ as *mut libc::sockaddr, &mut sl) != 0 || sa.ss_family as i32 != libc::AF_INET {
                continue;
            }
            let sin = &*(&sa as *const _ as *const libc::sockaddr_in);
            let ip = std::net::Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr));
            let addr = format!("{ip}:{}", u16::from_be(sin.sin_port));
            if addr != peer {
                continue;
            }
            // The runner's control (AK_RPC_PLANT=nagle): Nagle switched ON on one live socket
            // first, so the read-back must refuse the run.
            if n == 0 && std::env::var("AK_RPC_PLANT").map_or(false, |v| v == "nagle") {
                let off: libc::c_int = 0;
                libc::setsockopt(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, &off as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as libc::socklen_t);
            }
            let mut v: libc::c_int = 0;
            let mut vl = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
            if libc::getsockopt(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, &mut v as *mut _ as *mut libc::c_void, &mut vl) != 0 {
                continue;
            }
            n += 1;
            if v == 0 {
                bad.push(fd);
            }
        }
    }
    if n == 0 || !bad.is_empty() {
        eprintln!("REFUSED (CAMPAIGN 4.0, Nagle off on every socket): {n} TCP sockets to {peer} read back, Nagle ON on fds {bad:?}; no sample is taken");
        std::process::exit(7);
    }
    n
}

/// The header line of requirement 25 (D9), the same in every suite.
pub fn alloc_header(mode: &str, read: &str) -> String {
    format!("this process ran {mode} (a 16 MiB malloc read back {read}); modes: default = glibc's default allocator, GLIBC_TUNABLES unset (the main figures); pinned = GLIBC_TUNABLES={PINNED_TUNABLES} on the measured client only (the labelled diagnostic, files labelled alloc-pinned); every sample carries alloc and minflt (minor faults over its measured span, read outside the timers)")
}

/// criterion's measurement, replaced by process CPU time (requirement 21: criterion's
/// default is wall time). One value per criterion sample (= round), in ns.
pub struct ProcessCpu;

pub struct NsFormatter;

impl ValueFormatter for NsFormatter {
    fn scale_values(&self, _typical: f64, _values: &mut [f64]) -> &'static str {
        "ns (process CPU)"
    }
    fn scale_throughputs(&self, _t: f64, _th: &Throughput, _v: &mut [f64]) -> &'static str {
        "ns (process CPU)"
    }
    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        "ns"
    }
}

impl Measurement for ProcessCpu {
    type Intermediate = u64;
    type Value = u64;
    fn start(&self) -> u64 {
        process_clock_ns()
    }
    fn end(&self, i: u64) -> u64 {
        process_clock_ns() - i
    }
    fn add(&self, a: &u64, b: &u64) -> u64 {
        a + b
    }
    fn zero(&self) -> u64 {
        0
    }
    fn to_f64(&self, v: &u64) -> f64 {
        *v as f64
    }
    fn formatter(&self) -> &dyn ValueFormatter {
        &NsFormatter
    }
}

#[inline(always)]
pub fn mix(h: u64, x: u64) -> u64 {
    h.rotate_left(5) ^ x.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// What one root offers each arm. Rendered per root by `gen/rust_campaign.py`.
pub trait Ops {
    const ROOT: &'static str;
    type F: prost::Message + Default + Clone + PartialEq + std::fmt::Debug + 'static;
    type P: prost::Message + Default + Clone + PartialEq + std::fmt::Debug + 'static;
    fn build(pid: &str) -> Option<Self::F>;
    fn n_decode(b: &[u8], retain: bool) -> Result<Self::F, i32>;
    fn n_encode(v: &Self::F, e: &mut ak_rt::Enc, retain: bool);
    /// The push family (`core-ffi-push` since D24; the RPC cells' push twins do not exist).
    fn f_decode(c: &Ctx, b: &[u8], retain: bool) -> Result<Self::F, i32>;
    fn f_encode(c: &Ctx, v: &Self::F, retain: bool) -> Result<usize, i32>;
    fn f_pull(c: &Ctx, b: &[u8], retain: bool, toks: &mut Vec<i64>) -> Result<Self::F, i32>;
    /// FIX-PLAN D23/D24: the FSM decode family, the target's decode (`core-ffi`, and the
    /// core-codec RPC cells' response decode).
    fn f_fsm(c: &Ctx, b: &[u8], retain: bool, toks: &mut Vec<i64>) -> Result<Self::F, i32>;
    /// D23 attribution probe (drop mode): the FSM's events collected into a pull-format
    /// buffer, then the pull family's replay. Not a campaign arm.
    fn f_fsm_collect(c: &Ctx, b: &[u8], toks: &mut Vec<i64>, buf: &mut Vec<u64>) -> Result<Self::F, i32>;
    /// Optimisation Z1 (labelled extra arm `core-ffi-zc`): `bytes` fields share `b`.
    fn f_decode_zc(c: &Ctx, b: &bytes::Bytes, retain: bool) -> Result<Self::F, i32>;
    /// Decision 11 rule 6: this root's (bound) decode context.
    fn dec_ctx(c: &Ctx) -> *mut ak_abi::ak_dec_ctx;
    fn touch_f(v: &Self::F) -> u64;
    fn touch_p(v: &Self::P) -> u64;
}

pub trait Visit {
    fn visit<R: Ops>(&mut self);
}

/// CAMPAIGN.md req 10's unknown-field modes of core-native, core-ffi and core-ffi-pull in
/// THIS build: `drop` and `retain` with `unknown-fields`, `no-unknown` (support compiled
/// out: a separate build and binary) without. (label, retain?)
#[cfg(feature = "unknown-fields")]
pub const MODES: &[(&str, bool)] = &[("drop", false), ("retain", true)];
#[cfg(not(feature = "unknown-fields"))]
pub const MODES: &[(&str, bool)] = &[("no-unknown", false)];

/// Which fill the core-ffi encode arm (and the RPC cells C and D) use, for every header.
pub const FFI_ENCODE_FILL: &str = "sparse (ABI v1 decision 9: top-level element groups cleared, min(n, chunk) of them, then only non-default fields written; binding encode_into_<root>_zeroed / _unk_zeroed; nested groups and the root group keep the total fill)";

/// Requirement 11's encode variants: (end state, input). incumbent-prod and armonik have
/// `reused-buffer` and `transport-ready-tonic`; core-native and core-ffi have a third,
/// `transport-ready-core` (`VARIANTS_CORE`), because their RPC paths hand two transports
/// two different forms.
pub const VARIANTS: &[(&str, &str)] = &[
    ("reused-buffer", "hot"),
    ("reused-buffer", "pool"),
    ("transport-ready-tonic", "hot"),
    ("transport-ready-tonic", "pool"),
];
/// core-native's and core-ffi's variants: `VARIANTS` plus the core-transport form.
pub const VARIANTS_CORE: &[(&str, &str)] = &[
    ("reused-buffer", "hot"),
    ("reused-buffer", "pool"),
    ("transport-ready-core", "hot"),
    ("transport-ready-core", "pool"),
    ("transport-ready-tonic", "hot"),
    ("transport-ready-tonic", "pool"),
];
/// What each transport-ready row is, for every header.
pub const TRANSPORT_FORMS: &str = "transport-ready-tonic = the Bytes the arm hands tonic: incumbent-prod and armonik a frozen Bytes split from a reused BytesMut (cell A); core-ffi ak_enc_take_owned's buffer wrapped by Bytes::from_owner, no copy, released with ak_bytes_free when dropped (cell D; optimisation T1); core-native Enc::take, its buffer moved into a Bytes (from_owner) and recycled when dropped, O(1) (cell F; optimisation T1). transport-ready-core = the form the arm hands the core's transport: core-ffi's is the encode context itself (cell C: ak_call_unary_enc MOVES the core's buffer into the request inside the call, no host copy), core-native's its reused Enc buffer (cell E: ak_call_unary copies it inside the call); either way the host does nothing after the encode, so the op is the reused-buffer op, timed as its own row (an in-process repeat of reused-buffer), and the move or copy is inside the RPC call's time";
/// D24: core-ffi decodes with the FSM; core-ffi-push and core-ffi-pull are labelled extra
/// decode arms in the same randomised blocks.
pub const ARMS: [&str; 6] = ["incumbent-prod", "armonik", "core-native", "core-ffi", "core-ffi-push", "core-ffi-pull"];

/// Requirement 22 as amended 2026-09-26 (FIX-PLAN R-H23): the order is RANDOMISED per
/// launch, as far as the engine allows. Criterion runs benchmarks in the order they are
/// registered and has no shuffle of its own, so the suite registers them in a seeded random
/// order: arm blocks permuted, and the cases inside each block permuted. The seed is the
/// launch number, so a launch's order is reproducible and is written into its header.
pub fn arm_order(launch: usize) -> Vec<&'static str> {
    let mut v = ARMS.to_vec();
    shuffle(&mut v, launch as u64);
    v
}

/// FIX-PLAN D24 (owner, 2026-10-09): the FSM is the target decode family, so the D23 extra
/// arm `core-ffi-fsm` (AK_FSM=1) is gone: `core-ffi` IS the FSM, and push is the labelled
/// extra `core-ffi-push`. AK_FSM is refused rather than silently ignored, so a D23 script run
/// against this harness fails instead of timing arms other than the ones it names.
pub fn refuse_ak_fsm() {
    if std::env::var_os("AK_FSM").is_some() {
        panic!("AK_FSM is retired by D24: core-ffi decodes with the FSM, core-ffi-push is push (unset AK_FSM)");
    }
}

/// A deterministic Fisher-Yates shuffle (splitmix64 from `seed`): the same seed gives the
/// same order on every machine.
pub fn shuffle<T>(v: &mut [T], seed: u64) {
    let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    let mut next = || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    for i in (1..v.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

/// One timed case: `op` runs ONE operation and returns something to black-box.
pub struct Case {
    pub arm: &'static str,
    pub dir: &'static str,
    pub payload: String,
    pub content: &'static str,
    pub unknown_mode: &'static str,
    /// Requirement 11 (R-H29), encode rows only: `reused-buffer` or `transport-ready`.
    pub end_state: &'static str,
    /// Requirement 11, encode rows only: `hot` (one graph) or `pool` (distinct graphs
    /// beyond the last-level cache).
    pub input: &'static str,
    pub op: Box<dyn FnMut() -> u64>,
    /// Built before the case's warm-up and freed after its measurement (the pool): graph
    /// construction outside every timed window, and one pool alive at a time.
    pub prep: Option<Box<dyn FnMut()>>,
    pub done: Option<Box<dyn FnMut()>>,
    /// A pool case's (graphs, heap bytes they hold), measured when `prep` built it.
    pub pool_info: Option<std::rc::Rc<std::cell::Cell<(usize, usize)>>>,
}

/// Requirement 11's pool input: at least this many wire bytes of distinct graphs.
/// `AK_POOL_BYTES`, else twice `AK_LLC_BYTES` (the last-level cache, 13.75 MiB on the
/// reference i9-7900X by default); both from the environment, stated in the header.
pub fn llc_bytes() -> usize {
    std::env::var("AK_LLC_BYTES").ok().and_then(|v| v.parse().ok()).unwrap_or(14_417_920)
}
pub fn pool_bytes() -> usize {
    std::env::var("AK_POOL_BYTES").ok().and_then(|v| v.parse().ok()).unwrap_or(2 * llc_bytes())
}
/// The heap bytes in use (glibc `mallinfo2`: arena bytes in use plus mmapped blocks). Read
/// only while a pool is built, never inside a timed window.
pub fn heap_in_use() -> usize {
    let m = unsafe { libc::mallinfo2() };
    m.uordblks + m.hblkhd
}

/// Graphs are cloned into a pool until the HEAP they hold reaches `pool_bytes()` (measured
/// with `heap_in_use`, so the pool is beyond the last-level cache by what it occupies, not
/// by an estimate from its wire size), at least 2 and at most 2^20 graphs.
pub const POOL_MAX: usize = 1 << 20;

/// A pool of distinct graphs, built by `prep`, read by the op, freed by `done`.
struct Pool<T>(std::rc::Rc<std::cell::UnsafeCell<Vec<T>>>);
impl<T> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Pool(self.0.clone())
    }
}
impl<T: Clone + 'static> Pool<T> {
    fn new() -> Self {
        Pool(std::rc::Rc::new(std::cell::UnsafeCell::new(Vec::new())))
    }
    fn hooks(&self, v: &'static T, info: std::rc::Rc<std::cell::Cell<(usize, usize)>>) -> (Box<dyn FnMut()>, Box<dyn FnMut()>) {
        let (a, b) = (self.clone(), self.clone());
        (Box::new(move || unsafe {
            let target = pool_bytes();
            let h0 = heap_in_use();
            let p = &mut *a.0.get();
            *p = Vec::new();
            loop {
                for _ in 0..64 {
                    p.push(v.clone());
                }
                let held = heap_in_use().saturating_sub(h0);
                if (held >= target && p.len() >= 2) || p.len() >= POOL_MAX {
                    info.set((p.len(), held));
                    break;
                }
            }
         }),
         Box::new(move || unsafe { *b.0.get() = Vec::new() }))
    }
    #[inline(always)]
    fn get(&self, i: usize) -> &T {
        unsafe {
            let v = &*self.0.get();
            v.get_unchecked(i % v.len())
        }
    }
}

/// One input of the codec suite: a payload (with a content set) or a corpus `U-*` row.
pub struct Input {
    pub id: String,
    pub root: String,
    pub content: &'static str,
    /// Wire bytes every decode arm reads.
    pub bytes: Vec<u8>,
    /// Encode is measured (false for P7.1: no canonical writer can produce it, SHAPES.md).
    pub encode: bool,
    /// A `U-*` row: the encode value is each arm's own decode of the row, per mode.
    pub unknown_row: bool,
    /// For the manifest check (ASCII payloads).
    pub sha256: Option<String>,
    /// A `U-*` row's accepted re-encodings (sha256), from the corpus manifest.
    pub accepted: Vec<String>,
}

pub fn ffi_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..").canonicalize().unwrap()
}

pub fn sha(b: &[u8]) -> String {
    harness::manifest::sha(b)
}

/// Requirement 7: the 16 payloads (ASCII), the latin1 and wide content sets on P1.2, P2.2
/// and P2.4, and every accepted, non-disputed corpus `U-*` row whose root is one of the
/// shapes core's 7 ABI roots (92 rows; the owner's R-H27 answer).
/// `only`: comma-separated id prefixes (smoke runs narrow a launch).
pub fn inputs(only: &[String]) -> Vec<Input> {
    use shapes_values::{set_content_set, ContentSet};
    let keep = |id: &str| only.is_empty() || only.iter().any(|o| id.starts_with(o.as_str()));
    let man = harness::manifest::Manifest::load();
    let mut out = Vec::new();
    let sets: [(ContentSet, &'static str); 3] =
        [(ContentSet::Ascii, "ascii"), (ContentSet::Latin1, "latin1"), (ContentSet::Wide, "wide")];
    for (pid, row) in &man.0 {
        for (cs, cname) in sets {
            // Requirement 7 (R-H26): Latin-1 and wide on P1.2, P2.2 and P2.4.
            if cname != "ascii" && pid != "P1.2" && pid != "P2.2" && pid != "P2.4" {
                continue;
            }
            let id = if cname == "ascii" { pid.clone() } else { format!("{pid}/{cname}") };
            if !keep(&id) {
                continue;
            }
            set_content_set(cs);
            let root = root_of(pid);
            let bytes = if pid == "P7.1" {
                row.vector.clone().expect("P7.1 vector")
            } else {
                let mut b = None;
                struct Enc<'a>(&'a str, &'a mut Option<Vec<u8>>);
                impl Visit for Enc<'_> {
                    fn visit<R: Ops>(&mut self) {
                        let v = R::build(self.0).expect("payload builder");
                        let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
                        R::n_encode(&v, &mut e, false);
                        *self.1 = Some(e.buf.to_vec());
                    }
                }
                generated::roots::with_root(&root, &mut Enc(pid, &mut b));
                b.unwrap()
            };
            set_content_set(ContentSet::Ascii);
            out.push(Input {
                id,
                root,
                content: cname,
                bytes,
                encode: pid != "P7.1",
                unknown_row: false,
                sha256: if cname == "ascii" { Some(row.sha256.clone()) } else { None },
                accepted: Vec::new(),
            });
        }
    }
    let cdir = ffi_dir().join("corpus/generated");
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cdir.join("manifest.json")).unwrap()).unwrap();
    for (id, r) in m["vectors"].as_object().unwrap() {
        if r["class"] != "unknown" || r["verdict"] == "disputed" || r["expect"] != "accept" {
            continue;
        }
        let root = r["root"].as_str().unwrap();
        if !generated::roots::ROOTS.contains(&root) || !keep(id) {
            continue;
        }
        out.push(Input {
            id: id.clone(),
            root: root.to_string(),
            content: "ascii",
            bytes: std::fs::read(cdir.join(r["file"].as_str().unwrap())).unwrap(),
            encode: true,
            unknown_row: true,
            sha256: None,
            accepted: r["accepted_encodings"].as_array().map(|a| a.iter()
                .filter_map(|e| e["sha256"].as_str().map(String::from)).collect()).unwrap_or_default(),
        });
    }
    out
}

fn root_of(pid: &str) -> String {
    let v: serde_json::Value = serde_json::from_slice(
        &std::fs::read(ffi_dir().join("schema/generated/manifest.json")).unwrap(),
    )
    .unwrap();
    v["payloads"][pid]["root"].as_str().unwrap().to_string()
}

/// Every case of one input, for root `R`. The objects an encode case writes are built here,
/// once, outside every timed region; Rust's codecs keep no per-instance size memo
/// (requirement 11: prost recomputes `encoded_len` on every encode), so re-encoding the same
/// graph is a fresh serialisation each iteration.
pub fn cases_for<R: Ops>(ctx: &'static Ctx, inp: &Input, zc: bool) -> Vec<Case> {
    use bytes::{Bytes, BytesMut};
    use prost::Message;
    let mut out = Vec::new();
    let wire: &'static [u8] = Box::leak(inp.bytes.clone().into_boxed_slice());
    let wire_b = Bytes::from_static(wire);
    let (content, pid) = (inp.content, inp.id.clone());
    let mut push_full = |arm: &'static str, dir: &'static str, mode: &'static str, end_state: &'static str,
                         input: &'static str, op: Box<dyn FnMut() -> u64>,
                         hooks: Option<(Box<dyn FnMut()>, Box<dyn FnMut()>)>,
                         info: Option<std::rc::Rc<std::cell::Cell<(usize, usize)>>>| {
        let pool_info = if hooks.is_some() { info } else { None };
        let (prep, done) = match hooks { Some((p, d)) => (Some(p), Some(d)), None => (None, None) };
        out.push(Case { arm, dir, payload: pid.clone(), content, unknown_mode: mode, end_state, input, op, prep, done, pool_info });
    };
    // The objects each encode arm writes: for a payload, the builder's value (the prost arm
    // gets prost's decode of its canonical bytes); for a U-* row, each arm's own decode.
    // On a `U-*` row prost may REFUSE what protobuf accepts (a known field number at a
    // foreign wire type is an error in prost, an unknown field in protobuf): that arm is
    // then not timed on that row, and `refusals` names it in the log header.
    let p_ok = R::P::decode(wire).ok();
    let a_ok = R::F::decode(wire).ok();
    let (p_run, a_run) = (p_ok.is_some(), a_ok.is_some());
    let p_val: &'static R::P = Box::leak(Box::new(p_ok.unwrap_or_default()));
    let a_val: &'static R::F = Box::leak(Box::new(a_ok.unwrap_or_default()));
    let f_val = |retain: bool| -> &'static R::F {
        Box::leak(Box::new(if inp.unknown_row {
            R::n_decode(wire, retain).expect("core-native decodes the row")
        } else {
            R::n_decode(wire, false).expect("core-native decodes the payload")
        }))
    };
    let modes: &'static [(&'static str, bool)] = MODES;
    // Requirement 11 (R-H29): every encode arm in labelled variants, graph construction
    // outside the timed window:
    //   end state  reused-buffer          the bytes left in a buffer the arm reuses (no allocation)
    //              transport-ready-tonic  the form the arm's RPC path hands tonic: a frozen
    //                                     `Bytes` split from a reused `BytesMut` (tonic's encode
    //                                     buffer) for incumbent-prod and armonik (cell A); for
    //                                     core-native and core-ffi what cells F and D hand tonic
    //              transport-ready-core   core-native and core-ffi: what cells E and C hand the
    //                                     core's transport -- the reused buffer (E, copied inside
    //                                     ak_call_unary) or the encode context (C, its buffer moved
    //                                     inside ak_call_unary_enc): nothing after the encode on
    //                                     the host, the reused-buffer op timed as its own row
    //   input      hot   one graph re-encoded; pool  distinct graphs, in turn, cloned until
    //                    the heap they hold reaches `pool_bytes()` (`Pool::hooks`)
    if inp.encode && p_run {
        for (end, input) in VARIANTS {
            let pool = Pool::<R::P>::new();
            let info = std::rc::Rc::new(std::cell::Cell::new((0, 0)));
            let hooks = (*input == "pool").then(|| pool.hooks(p_val, info.clone()));
            let mut buf = BytesMut::with_capacity(wire.len() * 2 + 64);
            let tr = *end == "transport-ready-tonic";
            let hot = *input == "hot";
            let mut i = 0usize;
            push_full("incumbent-prod", "encode", "default", end, input, Box::new(move || {
                let v = if hot { p_val } else { i += 1; pool.get(i) };
                if tr {
                    buf.reserve(v.encoded_len());
                    v.encode(&mut buf).unwrap();
                    let b = buf.split().freeze();
                    b.len() as u64
                } else {
                    buf.clear();
                    v.encode(&mut buf).unwrap();
                    buf.len() as u64
                }
            }), hooks, Some(info));
        }
    }
    if inp.encode && a_run {
        for (end, input) in VARIANTS {
            let pool = Pool::<R::F>::new();
            let info = std::rc::Rc::new(std::cell::Cell::new((0, 0)));
            let hooks = (*input == "pool").then(|| pool.hooks(a_val, info.clone()));
            let mut buf = BytesMut::with_capacity(wire.len() * 2 + 64);
            let tr = *end == "transport-ready-tonic";
            let hot = *input == "hot";
            let mut i = 0usize;
            push_full("armonik", "encode", "default", end, input, Box::new(move || {
                let v = if hot { a_val } else { i += 1; pool.get(i) };
                if tr {
                    buf.reserve(v.encoded_len());
                    v.encode(&mut buf).unwrap();
                    let b = buf.split().freeze();
                    b.len() as u64
                } else {
                    buf.clear();
                    v.encode(&mut buf).unwrap();
                    buf.len() as u64
                }
            }), hooks, Some(info));
        }
    }
    if inp.encode {
        for &(mname, retain) in modes {
            for (end, input) in VARIANTS_CORE {
                let tr = *end == "transport-ready-tonic";
                let hot = *input == "hot";
                let v = f_val(retain);
                let pool = Pool::<R::F>::new();
                let info = std::rc::Rc::new(std::cell::Cell::new((0, 0)));
                let hooks = (*input == "pool").then(|| pool.hooks(v, info.clone()));
                let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
                let mut i = 0usize;
                push_full("core-native", "encode", mname, end, input, Box::new(move || {
                    let x = if hot { v } else { i += 1; pool.get(i) };
                    R::n_encode(x, &mut e, retain);
                    // T1: cell F's form is the Enc's buffer moved into a Bytes (Enc::take)
                    if tr { e.take().len() as u64 } else { e.buf.len() as u64 }
                }), hooks, Some(info));
            }
            for (end, input) in VARIANTS_CORE {
                let tr = *end == "transport-ready-tonic";
                let hot = *input == "hot";
                let v = f_val(retain);
                let pool = Pool::<R::F>::new();
                let info = std::rc::Rc::new(std::cell::Cell::new((0, 0)));
                let hooks = (*input == "pool").then(|| pool.hooks(v, info.clone()));
                let mut i = 0usize;
                push_full("core-ffi", "encode", mname, end, input, Box::new(move || {
                    let x = if hot { v } else { i += 1; pool.get(i) };
                    let n = R::f_encode(ctx, x, retain).expect("core-ffi encode") as u64;
                    if tr {
                        // cell D (T1): the core's buffer moved to the host as an owned Bytes
                        ffi_owned_body(ctx.enc).expect("ak_enc_take_owned").len() as u64
                    } else {
                        // reused-buffer, and cell C's form (the context, moved inside the call)
                        n
                    }
                }), hooks, Some(info));
            }
        }
    }
    let mut push = |arm: &'static str, dir: &'static str, mode: &'static str, op: Box<dyn FnMut() -> u64>| {
        push_full(arm, dir, mode, "", "", op, None, None);
    };
    for (dir, read) in [("decode", false), ("decode-read", true)] {
        let b = wire_b.clone();
        if p_run {
            push("incumbent-prod", dir, "default", Box::new(move || {
                let v = R::P::decode(&mut b.clone()).unwrap();
                if read { R::touch_p(&v) } else { std::hint::black_box(&v); 0 }
            }));
        }
        let b = wire_b.clone();
        if a_run {
            push("armonik", dir, "default", Box::new(move || {
                let v = R::F::decode(&mut b.clone()).unwrap();
                if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
            }));
        }
        for &(mname, retain) in modes {
            push("core-native", dir, mname, Box::new(move || {
                let v = R::n_decode(wire, retain).unwrap();
                if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
            }));
            // D24: core-ffi decodes with the FSM family; push is the labelled extra.
            let mut toks = Vec::new();
            push("core-ffi", dir, mname, Box::new(move || {
                let v = R::f_fsm(ctx, wire, retain, &mut toks).unwrap();
                if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
            }));
            push("core-ffi-push", dir, mname, Box::new(move || {
                let v = R::f_decode(ctx, wire, retain).unwrap();
                if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
            }));
            let mut toks = Vec::new();
            push("core-ffi-pull", dir, mname, Box::new(move || {
                let v = R::f_pull(ctx, wire, retain, &mut toks).unwrap();
                if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
            }));
            if zc {
                // Optimisation Z1, a labelled extra arm (AK_ZC): `bytes` fields share the
                // input buffer instead of copying it (not decision 13's default).
                let bz = wire_b.clone();
                push("core-ffi-zc", dir, mname, Box::new(move || {
                    let v = R::f_decode_zc(ctx, &bz, retain).unwrap();
                    if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
                }));
            }
        }
    }
    out
}

/// Requirement 26, in the process that times: every timed encode arm's bytes against the
/// manifest (ASCII payloads) or against the incumbent's (content sets); every decode arm
/// decodes every input and the facade arms agree with each other within a mode; on a `U-*`
/// row the retain arms re-encode the row's bytes exactly and agree with each other.
/// Returns the failures, one line each.
pub fn precheck<R: Ops>(ctx: &Ctx, inp: &Input) -> (usize, Vec<String>, Vec<String>) {
    use prost::Message;
    let mut fails = Vec::new();
    let mut n = 0usize;
    let wire = &inp.bytes[..];
    let mut chk = |ok: bool, what: String| {
        n += 1;
        if !ok {
            fails.push(format!("{} {}: {}", inp.id, R::ROOT, what));
        }
    };
    let mut refused = Vec::new();
    let pv = R::P::decode(wire);
    let av = R::F::decode(wire);
    if inp.unknown_row {
        // The incumbent's reading of a U-* row is prost's, stated, not gated.
        if let Err(e) = &pv { refused.push(format!("{} incumbent-prod: {e}", inp.id)); }
        if let Err(e) = &av { refused.push(format!("{} armonik: {e}", inp.id)); }
    } else {
        chk(pv.is_ok(), "incumbent decodes".into());
        chk(av.is_ok(), "armonik decodes".into());
    }
    let mut toks = Vec::new();
    for &(_, retain) in MODES {
        let nv = R::n_decode(wire, retain);
        // D24: core-ffi's decode is the FSM family; core-ffi-push and core-ffi-pull are
        // labelled extras, each checked against it.
        let fv = R::f_fsm(ctx, wire, retain, &mut toks);
        let hv = R::f_decode(ctx, wire, retain);
        let pl = R::f_pull(ctx, wire, retain, &mut toks);
        chk(nv.is_ok() && fv.is_ok() && hv.is_ok() && pl.is_ok(), format!("native/ffi(fsm)/push/pull decode (retain={retain})"));
        chk(matches!((&fv, &hv), (Ok(a), Ok(b)) if format!("{a:?}") == format!("{b:?}")),
            format!("core-ffi-push == core-ffi (retain={retain})"));
        // D23: the FSM's event stream is pull's log record for record and the consumer's graph
        // is push's (the differential, every mode of this build).
        let (d, g) = harness::generated::binding::fsm_check_root(R::ROOT, ctx.dec, wire, retain, 0).expect("fsm root");
        chk(d.mismatch.is_none() && g.is_ok(), format!("fsm events == pull records (retain={retain}): {:?} {:?}", d.mismatch, g));
        // Optimisation Z1: the zero-copy decode gives the same value.
        let zv = R::f_decode_zc(ctx, &bytes::Bytes::copy_from_slice(wire), retain);
        chk(matches!((&fv, &zv), (Ok(a), Ok(b)) if format!("{a:?}") == format!("{b:?}")),
            format!("core-ffi-zc == core-ffi (retain={retain})"));
        if let (Ok(a), Ok(b), Ok(c)) = (&nv, &fv, &pl) {
            chk(format!("{a:?}") == format!("{b:?}") && format!("{a:?}") == format!("{c:?}"),
                format!("native == ffi == pull (retain={retain})"));
            // Encode arms, over this mode's value.
            if inp.encode {
                let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
                R::n_encode(a, &mut e, retain);
                let nb = e.buf.to_vec();
                let fb = R::f_encode(ctx, a, retain).map(|_| unsafe { harness::generated::binding::encoded(ctx.enc) }.to_vec());
                chk(fb.as_ref().map(|x| *x == nb).unwrap_or(false), format!("core-ffi encode == core-native (retain={retain})"));
                // T1: the moved forms carry the same bytes -- core-native's Enc::take (cell F)
                // and core-ffi's ak_enc_take_owned (cell D), twice each so the second take runs
                // on the recycled (or fresh) buffer; and a NULL out is refused.
                for _ in 0..2 {
                    R::n_encode(a, &mut e, retain);
                    chk(e.take()[..] == nb[..], format!("core-native Enc::take == encode (retain={retain})"));
                    let ob = R::f_encode(ctx, a, retain).ok().and_then(|_| ffi_owned_body(ctx.enc).ok());
                    chk(ob.as_ref().map(|b| b[..] == nb[..]).unwrap_or(false), format!("core-ffi ak_enc_take_owned == encode (retain={retain})"));
                }
                let _ = R::f_encode(ctx, a, retain);
                chk(unsafe { ak_abi::ak_enc_take_owned(ctx.enc, std::ptr::null_mut()) } == ak_abi::AK_ERR_INVALID_STATE,
                    "ak_enc_take_owned(NULL out) refused".to_string());
                if inp.unknown_row {
                    chk(inp.accepted.contains(&sha(&nb)),
                        format!("re-encode is one of the row's accepted encodings (retain={retain})"));
                }
                if !inp.unknown_row {
                    if let Some(s) = &inp.sha256 {
                        chk(sha(&nb) == *s, format!("core-native encode == manifest (retain={retain})"));
                    }
                }
            }
        }
    }
    if inp.encode && !inp.unknown_row {
        if let (Ok(p), Ok(a)) = (&pv, &av) {
            let pb = p.encode_to_vec();
            let ab = a.encode_to_vec();
            match &inp.sha256 {
                Some(s) => {
                    chk(sha(&pb) == *s, "incumbent encode == manifest".into());
                    chk(sha(&ab) == *s, "armonik encode == manifest".into());
                }
                None => {
                    chk(pb == wire, "incumbent encode == input (content set)".into());
                    chk(ab == wire, "armonik encode == incumbent (content set)".into());
                }
            }
        }
    }
    (n, fails, refused)
}

/// Requirement 27's header, as `# key: value` lines at the top of a JSON-lines log.
pub fn header(suite: &str, extra: &[(&str, String)]) -> Vec<String> {
    let mut h = vec![
        format!("# slice: rust   suite: {suite}"),
        "# INSTRUMENTATION unless run on the campaign machine (CAMPAIGN.md section 2); a container figure is not a result".into(),
        format!("# incumbent: prost 0.14 / tonic 0.14 (tonic-prost), as pinned in poc/rust/Cargo.lock"),
        format!("# build: release profile, core ak-core cdylib (shared, linked by the dynamic linker), features rpc,init-guard; harness guard on"),
        format!("# rustc: {}", option_env!("AK_RUSTC").unwrap_or("see run header")),
        format!("# decode UTF-8 check (the core's, utf8=\"reject\"): {} (ak-rt; optimisation C2 made simdutf8 the default)", ak_rt::strings::CHECK_UTF8),
    ];
    for (k, v) in extra {
        h.push(format!("# {k}: {v}"));
    }
    h
}
