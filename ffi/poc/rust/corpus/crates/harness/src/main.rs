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
    pub dec: *mut ak_dec_ctx,
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
    dec: fn(*mut ak_dec_ctx, &[u8]) -> Result<T, i32>,
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
    let cx = unsafe { Cx { enc: ak_enc_ctx_new(), dec: ak_dec_ctx_new(), tcs: binding::Tcs::trusted() } };
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
    let mut i = 1;
    while i < a.len() {
        match a[i].as_str() {
            "--manifest" => { manifest = PathBuf::from(&a[i + 1]); i += 1; }
            "--row" => { row = Some(a[i + 1].clone()); i += 1; }
            "--only" => { only = a[i + 1].split(',').map(String::from).collect(); i += 1; }
            "--timeout-ms" => { timeout = Duration::from_millis(a[i + 1].parse().unwrap()); i += 1; }
            x => panic!("unknown argument {x}"),
        }
        i += 1;
    }
    let manifest = manifest.canonicalize().expect("manifest path");
    std::process::exit(match row {
        Some(id) => child(&manifest, &id),
        None => parent(&manifest, timeout, &only),
    });
}
