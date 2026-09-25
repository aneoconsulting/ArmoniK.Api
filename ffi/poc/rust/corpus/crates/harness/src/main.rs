//! FIX-PLAN WP5 item 6.1: the conformance corpus (`ffi/corpus/CONTRACT.md`) through the C
//! ABI and core-native, with unknown fields DROPPED and RETAINED -- four arms, every row.
//!
//!   ffi-drop       the core generated for the corpus's reader schema, behind the C ABI,
//!                  `decode_with_*` / `encode_into_*` (no unknown-field capture)
//!   ffi-retain     the same core and binding, `decode_with_*_unk` / `encode_into_*_unk`
//!   native-drop    core-native rendered from the SAME plans, drop mode
//!   native-retain  core-native, retain mode
//!
//! Obligations run: C1 (parse every accept row), C2 (project it, `_unknown` excluded as the
//! contract allows), C3 (re-encode to one of `accepted_encodings`, or a re-ordering where
//! `permutation_accepted`; the FORM written is recorded), C4 (refuse every reject row and
//! record the code). C5 (produce) is not claimed: this harness builds no values of its own.
//! A disputed row is excluded from pass/fail and the reading produced is reported.
//!
//! Every row runs in a CHILD PROCESS under a timeout, so a hang (a wrapping length) or an
//! abort (a panic across `extern "C"`) is a row result and not the end of the run.
//!
//!   corpus [--manifest PATH] [--timeout-ms N]     the whole corpus, a verdict, exit 0/1
//!   corpus --row ID                                one row, one JSON line (the child)
//!   corpus --unk-controls [--plant]                decision 11's controls (WP5 step 7):
//!                                                  per-position discard on every accept
//!                                                  row the ABI carries, pull == push, and
//!                                                  the pre-allocated-buffer placement cases

use ak_abi::*;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

mod generated {
    pub mod binding;
    pub mod dispatch;
}
use generated::binding;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arm {
    FfiDrop,
    FfiRetain,
    NativeDrop,
    NativeRetain,
}
const ARMS: [(Arm, &str); 4] = [
    (Arm::FfiDrop, "ffi-drop"),
    (Arm::FfiRetain, "ffi-retain"),
    (Arm::NativeDrop, "native-drop"),
    (Arm::NativeRetain, "native-retain"),
];

pub struct Cx {
    pub enc: *mut ak_enc_ctx,
    pub dec: binding::DecCtxs,
    pub tcs: binding::Tcs,
}

pub enum Outcome {
    /// Decoded: the projection, and the re-encoding (or the re-encode's error).
    Ok(Value, Result<Vec<u8>, i32>),
    Err(i32),
    NotInAbi,
    UnknownRoot,
}

pub fn ffi<T>(
    b: &[u8],
    cx: &Cx,
    dec: fn(binding::DecCtxs, &[u8]) -> Result<T, i32>,
    enc: fn(*mut ak_enc_ctx, &T, &binding::Tcs) -> Result<usize, i32>,
    proj: fn(&T) -> Value,
) -> Outcome {
    match dec(cx.dec, b) {
        Err(e) => Outcome::Err(e),
        Ok(v) => {
            let p = proj(&v);
            let r = match enc(cx.enc, &v, &cx.tcs) {
                Ok(_) => Ok(unsafe { binding::encoded(cx.enc) }.to_vec()),
                Err(e) => Err(e),
            };
            Outcome::Ok(p, r)
        }
    }
}

pub fn native<T>(
    b: &[u8],
    dec: fn(&[u8]) -> Result<T, i32>,
    enc: fn(&T) -> Vec<u8>,
    proj: fn(&T) -> Value,
) -> Outcome {
    match dec(b) {
        Err(e) => Outcome::Err(e),
        Ok(v) => Outcome::Ok(proj(&v), Ok(enc(&v))),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}
fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}
fn sha(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

fn default_manifest() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.join("../../../../../corpus/generated/manifest.json")
}

fn load(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read manifest")).expect("manifest JSON")
}

// ------------------------------------------------------------------------ the child

fn child(manifest: &Path, id: &str) -> i32 {
    let m = load(manifest);
    let row = &m["vectors"][id];
    let dir = manifest.parent().unwrap();
    let bytes = std::fs::read(dir.join(row["file"].as_str().unwrap())).expect("vector file");
    let root = row["root"].as_str().unwrap();
    // R-G7: the binding renders `ak_init` (plan.lifecycle) and this build of the core has
    // `init-guard` ON, so skipping the call must fail every C ABI arm. AK_CORPUS_SKIP_INIT
    // is that control (gen/corpus.sh runs it and requires the failure).
    if std::env::var_os("AK_CORPUS_SKIP_INIT").is_none() {
        let rc = binding::ak_init_once();
        assert!(rc >= 0, "ak_init: {rc}");
    }
    let cx = unsafe { Cx { enc: ak_enc_ctx_new(), dec: binding::DecCtxs::new(), tcs: binding::Tcs::trusted() } };
    let plant = std::env::var("AK_CORPUS_PLANT").ok();
    let mut arms = Map::new();
    for (arm, name) in ARMS {
        let mut outcome = generated::dispatch::run(root, arm, &bytes, &cx);
        // The harness seen failing (AK_CORPUS_PLANT): a comparison nothing has ever made
        // fail is a comparison nobody has seen work. Each plant must turn rows red.
        match (plant.as_deref(), &mut outcome) {
            (Some("proj"), Outcome::Ok(p, _)) => { if let Value::Object(m) = p { m.insert("__planted".into(), Value::Bool(true)); } }
            (Some("reenc"), Outcome::Ok(_, Ok(re))) => re.push(0),
            (Some("accept"), Outcome::Err(_)) => outcome = Outcome::Ok(json!({}), Ok(Vec::new())),
            _ => {}
        }
        let v = match outcome {
            Outcome::Ok(p, Ok(re)) => json!({"ok": true, "proj": p, "hex": hex(&re)}),
            Outcome::Ok(p, Err(e)) => json!({"ok": true, "proj": p, "enc_err": e}),
            Outcome::Err(e) => json!({"ok": false, "err": e}),
            Outcome::NotInAbi => json!({"na": true}),
            Outcome::UnknownRoot => json!({"unknown_root": true}),
        };
        arms.insert(name.to_string(), v);
    }
    println!("{}", json!({"id": id, "arms": arms}));
    0
}

// ------------------------------------------------------------------------ the parent

/// The contract's `_unknown` is optional: stripped from the expectation before comparing.
fn strip_unknown(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter().filter(|(k, _)| k.as_str() != "_unknown").map(|(k, x)| (k.clone(), strip_unknown(x))).collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(strip_unknown).collect()),
        x => x.clone(),
    }
}

/// Top-level wire records (key bytes + value bytes), for the permutation rule.
fn records(b: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut d = ak_rt::dec::Dec::new(b);
    let mut out = Vec::new();
    while !d.at_end() {
        let s0 = d.pos;
        let k = d.varint();
        if d.err != 0 {
            return None;
        }
        d.skip((k >> 3) as u32, (k & 7) as u32);
        if d.err != 0 {
            return None;
        }
        out.push(b[s0..d.pos].to_vec());
    }
    out.sort();
    Some(out)
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
    disputed: usize,
    not_in_abi: usize,
    /// retain arms that wrote the unknown-DROPPED form on a row whose accepted forms include
    /// a retained one: a retention gap, not a contract failure (both forms are accepted).
    retain_gap: Vec<String>,
    forms: BTreeMap<String, usize>,
    fails: Vec<String>,
    disputes: Vec<String>,
    by_class: BTreeMap<String, (usize, usize)>,
}

fn eval_arm(row: &Value, dir: &Path, id: &str, arm: &str, r: &Value, vec_bytes: &[u8], t: &mut Tally) {
    let class = row["class"].as_str().unwrap_or("?").to_string();
    if r.get("na").is_some() {
        t.not_in_abi += 1;
        return;
    }
    let disputed = row["verdict"].as_str() == Some("disputed");
    let expect = row["expect"].as_str().unwrap();
    let mut why: Vec<String> = Vec::new();
    if r.get("timeout").is_some() || r.get("crash").is_some() {
        why.push(format!("child {}", r));
    } else if expect == "reject" {
        if r["ok"].as_bool() == Some(true) {
            why.push("C4: accepted a must-fail vector".into());
        }
    } else if r["ok"].as_bool() != Some(true) {
        why.push(format!("C1: refused with {}", r["err"]));
    } else {
        // C2
        if let Some(pf) = row["projection"].as_str() {
            let want = strip_unknown(&load(&dir.join(pf)));
            if want != r["proj"] {
                why.push(format!("C2: projection differs: got {} want {}", r["proj"], want));
            }
        }
        // C3
        match r.get("hex").and_then(|h| h.as_str()) {
            None => why.push(format!("C3: re-encode failed: {}", r["enc_err"])),
            Some(h) => {
                let re = unhex(h);
                let s = sha(&re);
                let forms = row["accepted_encodings"].as_array().cloned().unwrap_or_default();
                let hit = forms.iter().find(|f| f["sha256"].as_str() == Some(&s));
                match hit {
                    Some(f) => {
                        let label = f["forms"].as_array().map(|a| a.iter().map(|x| x.as_str().unwrap_or("")).collect::<Vec<_>>().join(" / ")).unwrap_or_default();
                        *t.forms.entry(label.clone()).or_default() += 1;
                        if arm.ends_with("retain") && class == "unknown" && label.contains("dropped") {
                            t.retain_gap.push(format!("{id} ({arm})"));
                        }
                    }
                    None => {
                        let perm_ok = row["permutation_accepted"].as_bool() == Some(true)
                            && records(&re).is_some()
                            && records(&re) == records(vec_bytes);
                        if perm_ok {
                            *t.forms.entry("a re-ordering of the committed form (permutation_accepted)".into()).or_default() += 1;
                        } else {
                            why.push(format!("C3: re-encoding {} ({} B) is not an accepted form", &s[..12], re.len()));
                        }
                    }
                }
            }
        }
    }
    if disputed {
        t.disputed += 1;
        let reading = if r["ok"].as_bool() != Some(true) {
            format!("refused ({})", r.get("err").cloned().unwrap_or(Value::Null))
        } else {
            let mut which = "none of the readings".to_string();
            if let Some(rs) = row["dispute"]["readings"].as_array() {
                for rd in rs {
                    if let Some(pf) = rd["projection"].as_str() {
                        if strip_unknown(&load(&dir.join(pf))) == r["proj"] {
                            which = format!("the reading of {}", rd["runtimes"]);
                        }
                    }
                }
            }
            format!("accepted; projection matches {which}")
        };
        t.disputes.push(format!("{id} [{arm}]: {reading}"));
        return;
    }
    let e = t.by_class.entry(class).or_default();
    if why.is_empty() {
        t.pass += 1;
        e.0 += 1;
    } else {
        t.fail += 1;
        e.1 += 1;
        t.fails.push(format!("{id} [{arm}]: {}", why.join("; ")));
    }
}

fn run_child(exe: &Path, manifest: &Path, id: &str, timeout: Duration) -> Value {
    use std::io::Read;
    let mut ch = Command::new(exe)
        .arg("--manifest").arg(manifest).arg("--row").arg(id)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("spawn child");
    // Drain both pipes on threads: a child whose output outgrows the pipe buffer would
    // otherwise block on write and read as a timeout (it did, on the large rows).
    let mut so = ch.stdout.take().unwrap();
    let mut se = ch.stderr.take().unwrap();
    let to = std::thread::spawn(move || { let mut v = Vec::new(); let _ = so.read_to_end(&mut v); v });
    let te = std::thread::spawn(move || { let mut v = Vec::new(); let _ = se.read_to_end(&mut v); v });
    let t0 = Instant::now();
    let status = loop {
        match ch.try_wait().expect("wait") {
            Some(st) => break Some(st),
            None => {
                if t0.elapsed() > timeout {
                    let _ = ch.kill();
                    let _ = ch.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    };
    let out = to.join().unwrap_or_default();
    let err = te.join().unwrap_or_default();
    match status {
        None => json!({"timeout": timeout.as_millis() as u64}),
        Some(st) if !st.success() => {
            let err = String::from_utf8_lossy(&err);
            let first = err.lines().find(|l| l.contains("panicked") || l.contains("cannot unwind")).unwrap_or("").to_string();
            json!({"crash": format!("{:?} {}", st.code(), first)})
        }
        Some(_) => serde_json::from_slice(&out).unwrap_or(json!({"crash": "bad child output"})),
    }
}

fn parent(manifest: &Path, timeout: Duration, only: &[String]) -> i32 {
    let m = load(manifest);
    let dir = manifest.parent().unwrap().to_path_buf();
    let rows = m["vectors"].as_object().unwrap();
    let exe = std::env::current_exe().unwrap();
    println!("# the conformance corpus, C ABI and core-native, unknown fields dropped and retained");
    println!("#   manifest   {} ({} rows)", manifest.display(), rows.len());
    println!("#   core       ak-core --features corpus,init-guard: the core generated for the corpus's");
    println!("#              READER schema by poc/codec/gen (plan.py + rust_abi.py); CONTRACT rule 0");
    println!("#   native     core_native / core_native_retain, rendered from the same plans");
    println!("#   decode utf8 policy: {}; recursion limit {}", facade::generated::core_native::UTF8_POLICY, facade::generated::core_native::LIMIT);
    println!("#   each row in a child process, timeout {} ms", timeout.as_millis());
    for (r, why) in generated::dispatch::NOT_IN_ABI {
        println!("#   NOT IN THE C ABI: {r}: {why}");
    }
    println!();
    let mut tallies: BTreeMap<&str, Tally> = ARMS.iter().map(|(_, n)| (*n, Tally::default())).collect();
    let mut hard = 0usize;
    if let Ok(p) = std::env::var("AK_CORPUS_PLANT") {
        println!("#   PLANTED DEFECT: {p} (a control run: it MUST fail)");
    }
    if !only.is_empty() {
        println!("#   rows limited to ids starting with {:?}", only);
    }
    for (id, row) in rows {
        if !only.is_empty() && !only.iter().any(|p| id.starts_with(p.as_str())) {
            continue;
        }
        let res = run_child(&exe, manifest, id, timeout);
        let vb = std::fs::read(dir.join(row["file"].as_str().unwrap())).unwrap_or_default();
        if res.get("timeout").is_some() || res.get("crash").is_some() {
            hard += 1;
            println!("!! {id}: {res}");
            for (_, n) in ARMS {
                eval_arm(row, &dir, id, n, &res, &vb, tallies.get_mut(n).unwrap());
            }
            continue;
        }
        for (_, n) in ARMS {
            eval_arm(row, &dir, id, n, &res["arms"][n], &vb, tallies.get_mut(n).unwrap());
        }
    }
    let mut total_fail = 0;
    for (_, n) in ARMS {
        let t = &tallies[n];
        total_fail += t.fail;
        println!("## {n}");
        println!("   pass {}  fail {}  disputed (excluded) {}  not in the C ABI {}", t.pass, t.fail, t.disputed, t.not_in_abi);
        for (c, (p, f)) in &t.by_class {
            println!("     {c:<10} pass {p:>4}  fail {f:>3}");
        }
        println!("   forms written (C3):");
        for (f, k) in &t.forms {
            println!("     {k:>4}  {f}");
        }
        for d in &t.disputes {
            println!("   disputed: {d}");
        }
        if !t.retain_gap.is_empty() {
            println!("   retain mode wrote the DROPPED form on {} row(s) (accepted by the contract; a retention gap):", t.retain_gap.len());
            for g in &t.retain_gap {
                println!("     {g}");
            }
        }
        for f in &t.fails {
            println!("   FAIL {f}");
        }
        println!();
    }
    println!("# rows that hung or crashed a child: {hard}");
    if total_fail == 0 && hard == 0 {
        println!("CORPUS PASSES on all four arms");
        0
    } else {
        println!("CORPUS FAILS: {total_fail} arm-row failure(s), {hard} hang/crash row(s)");
        1
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut manifest = default_manifest();
    let mut row: Option<String> = None;
    let mut timeout = Duration::from_millis(10_000);
    let mut only: Vec<String> = Vec::new();
    let (mut unkc, mut plant) = (false, false);
    let mut i = 1;
    while i < a.len() {
        match a[i].as_str() {
            "--manifest" => { manifest = PathBuf::from(&a[i + 1]); i += 1; }
            "--row" => { row = Some(a[i + 1].clone()); i += 1; }
            "--only" => { only = a[i + 1].split(',').map(String::from).collect(); i += 1; }
            "--timeout-ms" => { timeout = Duration::from_millis(a[i + 1].parse().unwrap()); i += 1; }
            "--unk-controls" => { unkc = true; }
            "--plant" => { plant = true; }
            x => panic!("unknown argument {x}"),
        }
        i += 1;
    }
    let manifest = manifest.canonicalize().expect("manifest path");
    if unkc {
        std::process::exit(unk_controls(&manifest, &only, plant));
    }
    std::process::exit(match row {
        Some(id) => child(&manifest, &id),
        None => parent(&manifest, timeout, &only),
    });
}


// ------------------------------------------------------------ decision 11's controls

/// WP5 step 7. (1) On every accept row whose root crosses the C ABI: each position's entry
/// zeroed in turn drops exactly that position (the all-armed value with that position's
/// bags cleared), the pull family delivers what push does, and map-entry bytes arrive
/// unless the entry position is zeroed. `--plant` skips the clearing, so (1) must FAIL on
/// the rows that carry unknowns. (2) A pre-allocated buffer is placed once: with no grow,
/// the second element that needs a buffer is refused (AK_ERR_CAPACITY) rather than given
/// the same one; with grow, it gets a different buffer.
fn unk_controls(manifest: &Path, only: &[String], plant: bool) -> i32 {
    let m = load(manifest);
    let dir = manifest.parent().unwrap();
    assert!(binding::ak_init_once() >= 0);
    let cx = unsafe { Cx { enc: ak_enc_ctx_new(), dec: binding::DecCtxs::new(), tcs: binding::Tcs::trusted() } };
    let (mut rows, mut with_unk, mut bad, mut pull_bad, mut entry_rows) = (0, 0, 0, 0, 0);
    let mut positions = 0usize;
    let mut changed = 0usize;
    for (id, row) in m["vectors"].as_object().unwrap() {
        if row["expect"] != "accept" || (!only.is_empty() && !only.iter().any(|o| id.starts_with(o.as_str()))) {
            continue;
        }
        let root = row["root"].as_str().unwrap();
        let bytes = std::fs::read(dir.join(row["file"].as_str().unwrap())).unwrap();
        let Some(r) = generated::dispatch::unk_controls(root, &cx, &bytes, plant) else { continue };
        let r = match r {
            Ok(r) => r,
            Err(e) => { println!("  ERR  {id}: {e}"); bad += 1; continue; }
        };
        rows += 1;
        positions += r.positions;
        changed += r.changed;
        if r.changed > 0 { with_unk += 1; }
        if r.entry_bytes > 0 { entry_rows += 1; }
        if !r.mismatched.is_empty() || !r.entry_mismatch.is_empty() {
            bad += 1;
            if bad <= 12 {
                println!("  FAIL {id} ({root}): positions {:?} did not drop exactly themselves; entry {:?}", r.mismatched, r.entry_mismatch);
            }
        }
        if !r.pull_equal { pull_bad += 1; println!("  FAIL {id}: pull != push"); }
        if id.starts_with("U-") && (r.changed > 0 || r.entry_bytes > 0) {
            println!("  {id:<34} {root:<26} positions {:>2}, changed by zeroing {:>2}, entry bytes {}", r.positions, r.changed, r.entry_bytes);
        }
    }
    println!("rows {rows} (accept, root in the ABI), {positions} (row, position) pairs; rows with unknowns at some position {with_unk} ({changed} pairs changed by zeroing); rows with map-entry bytes {entry_rows}");
    println!("discard mismatches: {bad} row(s); pull != push: {pull_bad} row(s){}", if plant { "   [PLANTED: clearing skipped]" } else { "" });
    let placed = if plant { true } else { prealloc_cases(&cx) };
    if bad == 0 && pull_bad == 0 && placed { 0 } else { 1 }
}

fn varint(mut n: u64, out: &mut Vec<u8>) {
    while n >= 0x80 { out.push((n as u8) | 0x80); n >>= 7; }
    out.push(n as u8);
}
fn ld(tag: u32, body: &[u8], out: &mut Vec<u8>) {
    varint(((tag as u64) << 3) | 2, out);
    varint(body.len() as u64, out);
    out.extend_from_slice(body);
}

/// Decision 11 rule 1's host side, instrumented: a grow that counts its calls (and its
/// FRESH calls, dst == NULL) through the ONE host pointer, then does what `unk_grow` does.
#[repr(C)]
#[derive(Default)]
struct GrowCount {
    calls: u32,
    fresh: u32,
}
unsafe extern "C" fn counting_grow(host: *mut std::ffi::c_void, want: i32, dst: *mut *mut u8, cap: *mut i32) -> i32 {
    let h = &mut *(host as *mut GrowCount);
    h.calls += 1;
    if (*dst).is_null() {
        h.fresh += 1;
    }
    binding::unk_grow(host, want, dst, cap)
}

const NOBUF: ak_unk_buf = ak_unk_buf { data: std::ptr::null_mut(), len: 0, cap: 0 };
fn alloc(n: usize) -> *mut u8 {
    unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align(n, 1).unwrap()) }
}
fn free(p: *mut u8, n: usize) {
    unsafe { std::alloc::dealloc(p, std::alloc::Layout::from_size_align(n, 1).unwrap()) }
}
fn verdict(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}

/// WP5 step 8: the pool, in-place refill, oneof switch and wrong-root controls.
fn prealloc_cases(cx: &Cx) -> bool {
    let mut ok = true;
    ok &= pool_cases(cx);
    ok &= refill_cases();
    ok &= oneof_cases(cx);
    ok &= wrong_root_cases(cx);
    ok
}

/// POOL (rule 1): a batched repeated position (ListResultsResponse.results, leaf elements)
/// with n pre-allocated buffers: no grow for the first n elements that carry unknowns,
/// then grow; with no grow, AK_ERR_CAPACITY. Taken buffers are cleared IN the host's struct.
fn pool_cases(cx: &Cx) -> bool {
    let mut b = Vec::new();
    let mut runs = Vec::new();
    for (i, sid) in [&b"aa"[..], b"bb", b"cc"].iter().enumerate() {
        let mut e = Vec::new();
        ld(1, sid, &mut e);
        let run = vec![0xa0, 0x06, 7 + i as u8]; // field 100, varint
        e.extend_from_slice(&run);
        runs.push(run);
        ld(1, &e, &mut b);
    }
    let mut ok = true;
    let pz = ak_unk_pool { bufs: std::ptr::null_mut(), n: 0, grow: None };
    for (name, grow) in [("pool n=2, grow", true), ("pool n=2, no grow", false)] {
        let (p0, p1) = (alloc(64), alloc(64));
        let mut arr = [ak_unk_buf { data: p0 as _, len: 0, cap: 64 }, ak_unk_buf { data: p1 as _, len: 0, cap: 64 }];
        let mut gc = GrowCount::default();
        let mut o = binding::unk_opts_list_results_response(Some(usize::MAX));
        o.host = &mut gc as *mut GrowCount as *mut std::ffi::c_void;
        o.self_ = ak_unk_opts { buf: NOBUF, grow: None };
        o.results_created_at = pz;
        o.results_completed_at = pz;
        o.results = ak_unk_pool { bufs: arr.as_mut_ptr(), n: 2, grow: if grow { Some(counting_grow) } else { None } };
        let r = binding::decode_with_list_results_response_opts(cx.dec, &b, &mut o);
        let cleared = arr.iter().all(|x| x.data.is_null());
        let line = match (&r, grow) {
            (Ok(v), true) => {
                let pp: Vec<*const u8> = v.results.iter().map(|e| e.unknown_fields.as_ptr()).collect();
                let bags = v.results.iter().zip(&runs).all(|(e, r)| &e.unknown_fields == r);
                let pass = bags && pp[0] == p0 as *const u8 && pp[1] == p1 as *const u8
                    && pp[2] != p0 as *const u8 && pp[2] != p1 as *const u8 && gc.calls == 1 && gc.fresh == 1 && cleared;
                ok &= pass;
                format!("elements 0,1 in the pool's buffers, element 2 from grow; grow calls {} (fresh {}); host entries cleared: {cleared}; bags exact: {bags}  {}",
                        gc.calls, gc.fresh, verdict(pass))
            }
            (Err(e), false) => {
                let pass = *e == AK_ERR_CAPACITY && cleared && gc.calls == 0;
                ok &= pass;
                // Rule 3: the taken buffers stayed the host's (not delivered); free them.
                free(p0, 64);
                free(p1, 64);
                format!("rc {e} (AK_ERR_CAPACITY when the pool is exhausted and there is no grow); host entries cleared: {cleared}  {}", verdict(pass))
            }
            (r, _) => { ok = false; format!("unexpected {:?}  FAIL", r.as_ref().map(|_| ())) }
        };
        println!("  placement: {name:<24} {line}");
    }
    ok
}

/// IN-PLACE REFILL (rule 1): ListTasksDetailedResponse.tasks is NOT batched (its element has
/// runs of its own), so the host refills the pool in `new_tasks`, writing its own struct;
/// the element decoded next must use the refilled buffer. Without the refill the second
/// element finds nothing and no grow: AK_ERR_CAPACITY.
struct RefillHost {
    opts: *mut ak_dec_ListTasksDetailedResponse_opts,
    arr: *mut ak_unk_buf,
    refill: bool,
    placed: Vec<*mut u8>,
    got: Vec<(*const u8, Vec<u8>)>,
    n: i64,
}
unsafe extern "C" fn r_new(_c: *mut ak_dec_ctx, obj: *mut std::ffi::c_void) -> i64 {
    let h = &mut *(obj as *mut RefillHost);
    if h.refill && (*h.arr).data.is_null() {
        let p = alloc(64);
        h.placed.push(p);
        *h.arr = ak_unk_buf { data: p as _, len: 0, cap: 64 };
        let _ = h.opts;
    }
    h.n += 1;
    h.n - 1
}
unsafe extern "C" fn r_apply(_c: *mut ak_dec_ctx, obj: *mut std::ffi::c_void, _t: i64, f: *const ak_dfix_TaskDetailed) {
    let h = &mut *(obj as *mut RefillHost);
    let u = (*f).unknown;
    let bytes = if u.data.is_null() { Vec::new() } else { std::slice::from_raw_parts(u.data as *const u8, u.len as usize).to_vec() };
    h.got.push((u.data as *const u8, bytes));
}

fn refill_cases() -> bool {
    use facade::generated::core_native_retain as nr;
    let mut v = facade::ListTasksDetailedResponse::default();
    let mut runs = Vec::new();
    for i in 0..3u8 {
        let mut t = facade::TaskDetailed::default();
        t.id = format!("t{i}");
        t.unknown_fields = vec![0xa0, 0x06, 20 + i];
        runs.push(t.unknown_fields.clone());
        v.tasks.push(t);
    }
    let b = nr::encode_list_tasks_detailed_response(&v);
    let mut ok = true;
    for refill in [true, false] {
        let p0 = alloc(64);
        let mut arr = [ak_unk_buf { data: p0 as _, len: 0, cap: 64 }];
        let mut o: ak_dec_ListTasksDetailedResponse_opts = unsafe { std::mem::zeroed() };
        o.tasks = ak_unk_pool { bufs: arr.as_mut_ptr(), n: 1, grow: None };
        let mut h = RefillHost { opts: &mut o, arr: arr.as_mut_ptr(), refill, placed: vec![p0], got: Vec::new(), n: 0 };
        let rc = unsafe {
            let ctx = ak_dec_ctx_new_ListTasksDetailedResponse(&mut o);
            let mut vt: ak_dvt_ListTasksDetailedResponse = std::mem::zeroed();
            vt.new_tasks = Some(r_new);
            vt.apply_tasks = Some(r_apply);
            let rc = ak_decode_ListTasksDetailedResponse(ctx, &mut h as *mut RefillHost as *mut std::ffi::c_void, b.as_ptr(), b.len(), &vt);
            ak_dec_ctx_free(ctx);
            rc
        };
        let line = if refill {
            let pass = rc == AK_OK && h.got.len() == 3
                && h.got.iter().zip(&h.placed).all(|((p, _), q)| *p == *q as *const u8)
                && h.got.iter().zip(&runs).all(|((_, g), r)| g == r) && h.placed.len() == 3;
            ok &= pass;
            format!("rc {rc}; 3 elements, each in the buffer the host wrote into its struct in new_tasks (refills {}), bags exact  {}",
                    h.placed.len() - 1, verdict(pass))
        } else {
            let pass = rc == AK_ERR_CAPACITY;
            ok &= pass;
            format!("rc {rc} (without the refill: AK_ERR_CAPACITY at the second element)  {}", verdict(pass))
        };
        for p in &h.placed {
            free(*p, 64);
        }
        println!("  placement: {:<24} {line}", if refill { "in-place refill" } else { "no refill (control)" });
    }
    ok
}

/// ONEOF (rule 4): one position, one buffer. A probe whose body switches message members
/// keeps ONE buffer (one fresh grow), emptied on each switch, so the final member's bag is
/// that member's run only.
fn oneof_cases(cx: &Cx) -> bool {
    let run = |x: u8| vec![0xa0u8, 0x06, x];
    let mut ok = true;
    for (name, seq, want_last) in [
        ("oneof stamp -> nothing", vec![(13u32, 1u8), (14, 2)], 2u8),
        ("oneof stamp -> nothing -> stamp", vec![(13, 1), (14, 2), (13, 3)], 3),
    ] {
        let mut p = Vec::new();
        ld(1, b"p", &mut p);
        for (tag, x) in &seq {
            ld(*tag, &run(*x), &mut p);
        }
        let mut b = Vec::new();
        ld(1, &p, &mut b);
        let mut gc = GrowCount::default();
        let mut o = binding::unk_opts_list_probe_response(None);
        o.host = &mut gc as *mut GrowCount as *mut std::ffi::c_void;
        o.probes_body.grow = Some(counting_grow);
        let r = binding::decode_with_list_probe_response_opts(cx.dec, &b, &mut o);
        let line = match r {
            Ok(v) => {
                let bag = match v.probes.first().and_then(|p| p.body.as_ref()) {
                    Some(facade::ProbeBody::AsStamp(t)) => Some(t.unknown_fields.clone()),
                    Some(facade::ProbeBody::AsNothing(e)) => Some(e.unknown_fields.clone()),
                    _ => None,
                };
                let pass = bag.as_deref() == Some(&run(want_last)[..]) && gc.fresh == 1;
                ok &= pass;
                format!("final member's bag {:?} (want its own run only); fresh buffers {}  {}", bag, gc.fresh, verdict(pass))
            }
            Err(e) => { ok = false; format!("rc {e}  FAIL") }
        };
        println!("  placement: {name:<24} {line}");
    }
    ok
}

/// ROOT-BOUND CONTEXTS (rule 6): a context bound to ListResultsResponse refuses another
/// root's decode, parse and reset; its own root still decodes after.
fn wrong_root_cases(cx: &Cx) -> bool {
    let ctx = cx.dec.list_results_response;
    let b: Vec<u8> = Vec::new();
    unsafe {
        let vt: ak_dvt_ListTasksDetailedResponse = std::mem::zeroed();
        let d = ak_decode_ListTasksDetailedResponse(ctx, std::ptr::null_mut(), b.as_ptr(), 0, &vt);
        let p = ak_parse_ListTasksDetailedResponse(ctx, b.as_ptr(), 0);
        let r = ak_dec_reset_ListTasksDetailedResponse(ctx, std::ptr::null_mut());
        let own = binding::decode_with_list_results_response(cx.dec, &b).is_ok();
        let pass = d == AK_ERR_INVALID_STATE && p == AK_ERR_INVALID_STATE && r == AK_ERR_INVALID_STATE && own;
        println!("  placement: {:<24} decode rc {d}, parse rc {p}, reset rc {r} (AK_ERR_INVALID_STATE = {AK_ERR_INVALID_STATE}); own root still decodes: {own}  {}",
                 "wrong root (refused)", verdict(pass));
        pass
    }
}
