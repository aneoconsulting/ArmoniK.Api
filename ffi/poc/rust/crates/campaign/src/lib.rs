//! FIX-PLAN WP3: the rust slice's campaign harness (design/CAMPAIGN.md).
//!
//! What lives here: the per-root arm table (`Ops`, rendered by `gen/rust_campaign.py`), the
//! case list of the codec suite (payloads, content sets, `U-*` rows; arms; directions;
//! unknown-field modes), the thread-CPU measurement criterion runs with, the JSON-lines
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
//!   core-ffi        the generated binding through the C ABI, push decode, drop and retain
//!   core-ffi-pull   the pull family (walk in place), decode only, drop and retain

pub mod generated {
    pub mod roots;
}

use criterion::measurement::{Measurement, ValueFormatter};
use criterion::Throughput;
use harness::arms::core_ffi_arm::Ctx;
use std::path::PathBuf;

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

/// criterion's measurement, replaced by thread CPU time (requirement 21: criterion's
/// default is wall time). One value per criterion sample, in ns.
pub struct ThreadCpu;

pub struct NsFormatter;

impl ValueFormatter for NsFormatter {
    fn scale_values(&self, _typical: f64, _values: &mut [f64]) -> &'static str {
        "ns (thread CPU)"
    }
    fn scale_throughputs(&self, _t: f64, _th: &Throughput, _v: &mut [f64]) -> &'static str {
        "ns (thread CPU)"
    }
    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        "ns"
    }
}

impl Measurement for ThreadCpu {
    type Intermediate = u64;
    type Value = u64;
    fn start(&self) -> u64 {
        thread_cpu_ns()
    }
    fn end(&self, i: u64) -> u64 {
        thread_cpu_ns() - i
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
    fn f_decode(c: &Ctx, b: &[u8], retain: bool) -> Result<Self::F, i32>;
    fn f_encode(c: &Ctx, v: &Self::F, retain: bool) -> Result<usize, i32>;
    fn f_pull(c: &Ctx, b: &[u8], retain: bool, toks: &mut Vec<i64>) -> Result<Self::F, i32>;
    fn touch_f(v: &Self::F) -> u64;
    fn touch_p(v: &Self::P) -> u64;
}

pub trait Visit {
    fn visit<R: Ops>(&mut self);
}

pub const ARMS: [&str; 5] = ["incumbent-prod", "armonik", "core-native", "core-ffi", "core-ffi-pull"];

/// Requirement 22: the arm order of launch `l` (1-based) is ARMS rotated by l - 1.
pub fn arm_order(launch: usize) -> Vec<&'static str> {
    let n = ARMS.len();
    (0..n).map(|i| ARMS[(i + launch.saturating_sub(1)) % n]).collect()
}

/// One timed case: `op` runs ONE operation and returns something to black-box.
pub struct Case {
    pub arm: &'static str,
    pub dir: &'static str,
    pub payload: String,
    pub content: &'static str,
    pub unknown_mode: &'static str,
    pub op: Box<dyn FnMut() -> u64>,
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

/// Requirement 7: the 16 payloads (ASCII), the latin1 and wide content sets on P1.2 and
/// P2.2, and every non-disputed corpus `U-*` row whose root this slice's ABI carries.
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
            if cname != "ascii" && pid != "P1.2" && pid != "P2.2" {
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
                        *self.1 = Some(e.buf.clone());
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
pub fn cases_for<R: Ops>(ctx: &'static Ctx, inp: &Input) -> Vec<Case> {
    use bytes::{Bytes, BytesMut};
    use prost::Message;
    let mut out = Vec::new();
    let wire: &'static [u8] = Box::leak(inp.bytes.clone().into_boxed_slice());
    let wire_b = Bytes::from_static(wire);
    let (content, pid) = (inp.content, inp.id.clone());
    let mut push = |arm: &'static str, dir: &'static str, mode: &'static str, op: Box<dyn FnMut() -> u64>| {
        out.push(Case { arm, dir, payload: pid.clone(), content, unknown_mode: mode, op });
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
    let modes: [(&'static str, bool); 2] = [("drop", false), ("retain", true)];
    if inp.encode && p_run {
        let mut buf = BytesMut::with_capacity(wire.len() * 2 + 64);
        push("incumbent-prod", "encode", "default", Box::new(move || {
            buf.clear();
            p_val.encode(&mut buf).unwrap();
            buf.len() as u64
        }));
    }
    if inp.encode && a_run {
        let mut buf = BytesMut::with_capacity(wire.len() * 2 + 64);
        push("armonik", "encode", "default", Box::new(move || {
            buf.clear();
            a_val.encode(&mut buf).unwrap();
            buf.len() as u64
        }));
    }
    if inp.encode {
        for (mname, retain) in modes {
            let v = f_val(retain);
            let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
            push("core-native", "encode", mname, Box::new(move || {
                R::n_encode(v, &mut e, retain);
                e.buf.len() as u64
            }));
            let v = f_val(retain);
            push("core-ffi", "encode", mname, Box::new(move || {
                R::f_encode(ctx, v, retain).expect("core-ffi encode") as u64
            }));
        }
    }
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
        for (mname, retain) in modes {
            push("core-native", dir, mname, Box::new(move || {
                let v = R::n_decode(wire, retain).unwrap();
                if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
            }));
            push("core-ffi", dir, mname, Box::new(move || {
                let v = R::f_decode(ctx, wire, retain).unwrap();
                if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
            }));
            let mut toks = Vec::new();
            push("core-ffi-pull", dir, mname, Box::new(move || {
                let v = R::f_pull(ctx, wire, retain, &mut toks).unwrap();
                if read { R::touch_f(&v) } else { std::hint::black_box(&v); 0 }
            }));
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
    for retain in [false, true] {
        let nv = R::n_decode(wire, retain);
        let fv = R::f_decode(ctx, wire, retain);
        let pl = R::f_pull(ctx, wire, retain, &mut toks);
        chk(nv.is_ok() && fv.is_ok() && pl.is_ok(), format!("native/ffi/pull decode (retain={retain})"));
        if let (Ok(a), Ok(b), Ok(c)) = (&nv, &fv, &pl) {
            chk(format!("{a:?}") == format!("{b:?}") && format!("{a:?}") == format!("{c:?}"),
                format!("native == ffi == pull (retain={retain})"));
            // Encode arms, over this mode's value.
            if inp.encode {
                let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
                R::n_encode(a, &mut e, retain);
                let nb = e.buf.clone();
                let fb = R::f_encode(ctx, a, retain).map(|_| unsafe { harness::generated::binding::encoded(ctx.enc) }.to_vec());
                chk(fb.as_ref().map(|x| *x == nb).unwrap_or(false), format!("core-ffi encode == core-native (retain={retain})"));
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
    ];
    for (k, v) in extra {
        h.push(format!("# {k}: {v}"));
    }
    h
}
