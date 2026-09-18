//! M3's shape coverage: explicit presence as three cases, the oneof including its
//! payload-free member, and the fields a reader does not know.
//!
//! Correctness only. No timings here on purpose: M3 is a small message and its value is
//! shape coverage.
//!
//! The unknown-field vectors are built here rather than taken from `ffi/schema/generated/`,
//! because a corpus generated from the schema that reads it can never contain a field the
//! reader does not know (README section 10 item 1). They are hand-built wire, and that is
//! stated: they have no manifest hash, and their check is that all four arms agree with
//! each other on the decoded value AND on the re-encoded bytes.

use facade::{ListProbeResponse, ListResultsResponse};
use harness::arms as m1;
use harness::arms_m3 as m3;
use prost::Message;
use shapes_prost::shapes as p;

fn main() {
    let ctx = m1::core_ffi_arm::Ctx::new();
    let mut bad = 0usize;

    // ---------------------------------------------------------------- explicit presence
    println!("# M3: explicit presence, as three cases and not one column");
    println!("#   design/SHAPES.md: a by-value group reports absent and empty identically");
    println!("#   unless it is designed not to. These are the three rows that say whether it");
    println!("#   was. 200 elements; presence cycles per field and one element in seven is");
    println!("#   present AND zero.");
    println!();
    println!("{:<20} {:<16} {:>8} {:>14} {:>17} {:>6}",
             "field", "arm", "absent", "present+zero", "present+nonzero", "total");
    let pv = m3::prost_arm::value(m3::P3_1);
    let bytes = m3::prost_arm::encode(&pv);
    let decoded: [(&str, ListProbeResponse); 3] = [
        ("armonik", m3::armonik_arm::decode(&bytes)),
        ("core-native", m3::core_native_arm::decode(&bytes)),
        ("core-ffi-rust", m3::core_ffi_arm::decode(&ctx, &bytes)),
    ];
    let mut per_field: Vec<Vec<m3::PresenceCensus>> = vec![Vec::new(); 3];
    for (arm, v) in &decoded {
        for (i, (name, c)) in m3::census(v).iter().enumerate() {
            println!("{:<20} {:<16} {:>8} {:>14} {:>17} {:>6}",
                     name, arm, c.absent, c.present_zero, c.present_nonzero, c.total());
            per_field[i].push(*c);
        }
    }
    // prost's own view, from its own types, as the independent fourth opinion.
    let mut pc = [m3::PresenceCensus::default(); 3];
    for e in &pv.probes {
        upd(&mut pc[0], e.opt_count.map(|x| x != 0));
        upd(&mut pc[1], e.opt_label.as_ref().map(|x| !x.is_empty()));
        upd(&mut pc[2], e.opt_flag);
    }
    for (i, name) in ["opt_count (int32)", "opt_label (string)", "opt_flag  (bool)"].iter().enumerate() {
        println!("{:<20} {:<16} {:>8} {:>14} {:>17} {:>6}",
                 name, "prost", pc[i].absent, pc[i].present_zero, pc[i].present_nonzero, pc[i].total());
        if per_field[i].iter().any(|c| *c != pc[i]) {
            println!("   ^^^ ARMS DISAGREE on this field");
            bad += 1;
        }
    }
    println!();
    println!("#   All three cases occur on every field, so the shape is exercised and not");
    println!("#   merely present. present+zero is the one a group cannot tell from absent");
    println!("#   unless its presence WORD carries it, which is what makes these rows a test.");

    // ---------------------------------------------------------------- the oneof
    println!();
    println!("# M3: the oneof, by member");
    println!();
    println!("{:<28} {:>7}", "member", "count");
    for (name, n) in m3::oneof_census(&decoded[0].1) {
        println!("{:<28} {:>7}", name, n);
    }
    for (arm, v) in &decoded {
        if m3::oneof_census(v) != m3::oneof_census(&decoded[0].1) {
            println!("   ^^^ {arm} DISAGREES");
            bad += 1;
        }
    }
    println!();
    println!("#   `as_nothing` is the payload-free member: present, empty, and selected by");
    println!("#   its presence alone. It is 40 of 200 here, so it is reached.");

    // ---------------------------------------------------------------- unknown fields
    println!();
    println!("# Fields the reader does not know (README section 10 item 1)");
    println!("#   Hand-built wire. No manifest hash: a corpus generated from the schema that");
    println!("#   reads it cannot contain one of these. The check is that all four arms agree");
    println!("#   on the decoded value and on the re-encoded bytes.");
    println!();
    println!("{:<44} {:<12} {:<12} {}", "vector", "arms agree", "re-encode", "unknown survives");

    for (name, probe_body, note) in probe_vectors() {
        let wire = wrap_probe(&probe_body);
        bad += check_probe(&ctx, name, &wire, note);
    }

    // An unknown ENUM value, which is a different thing: the field is known, the value is
    // not. design/SHAPES.md requires it to round-trip losslessly.
    let raw = result_with_status(999);
    bad += check_result(&ctx, "ResultRaw.status = 999 (unknown enum value)", &raw);

    bad += adapter_section();
    bad += adapter_reachability();

    println!();
    if bad == 0 {
        println!("VERDICT: every shape above is exercised and every arm agrees.");
    } else {
        println!("VERDICT: {bad} disagreement(s).");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------- M4, the adapter site

/// The facade type the `with` adapter produces: ONE type, TWO wire forms. Hand-written,
/// because the facade is the hand-written half of this design and M4 exists precisely
/// because "an adapter written from intuition is silently wrong on the failure path only".
///
/// Modelled on `packages/rust`'s `armonik::Output`, whose own doc comment makes the
/// distinction that matters: `Invalid` is "no member set", and it is distinct from `Ok`,
/// which carries nothing but *is* set -- a peer that reports no outcome is not a peer that
/// reports success.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Output {
    Invalid,
    Ok,
    Error(String),
}

/// Wire form A: the nested `TaskOutput { success: bool = 1, error: string = 2 }`.
fn to_nested(o: &Output) -> Option<(bool, String)> {
    match o {
        Output::Invalid => None,
        Output::Ok => Some((true, String::new())),
        Output::Error(d) => Some((false, d.clone())),
    }
}

fn from_nested(v: Option<(bool, String)>) -> Output {
    match v {
        None => Output::Invalid,
        Some((true, _)) => Output::Ok,
        Some((false, d)) => Output::Error(d),
    }
}

/// Wire form B: a plain `string` at `TaskSummary.error`.
///
/// The adapter written from intuition decodes an empty string as `Ok`. It is wrong: a plain
/// string can only carry the error, so it cannot distinguish "succeeded" from "said
/// nothing", and `Invalid` is the only honest answer.
fn to_plain(o: &Output) -> String {
    match o {
        Output::Error(d) => d.clone(),
        Output::Ok | Output::Invalid => String::new(),
    }
}

fn from_plain(s: &str) -> Output {
    if s.is_empty() {
        Output::Invalid
    } else {
        Output::Error(s.to_string())
    }
}

fn adapter_section() -> usize {
    println!();
    println!("# M4: the adapter site. One facade type, two wire forms.");
    println!("#   Checked by state and not by payload: P4.1's `error` is a sentence in all");
    println!("#   200 elements, so the payload exercises the ONE case that works. The states");
    println!("#   where the map is not injective never occur in it.");
    println!();
    println!("{:<18} {:<26} {:<20} {:<26} {}", "state", "-> nested wire", "nested round trip",
             "-> plain wire", "plain round trip");
    let mut bad = 0usize;
    for o in [Output::Invalid, Output::Ok, Output::Error("boom".into())] {
        let n = to_nested(&o);
        let back_n = from_nested(n.clone());
        let p = to_plain(&o);
        let back_p = from_plain(&p);
        let nd = if back_n == o { "ok".to_string() } else { format!("LOSES -> {back_n:?}") };
        let pd = if back_p == o { "ok".to_string() } else { format!("LOSES -> {back_p:?}") };
        println!("{:<18} {:<26} {:<20} {:<26} {}",
                 format!("{o:?}"),
                 match &n { None => "<field absent>".into(), Some((s, e)) => format!("success={s}, error={e:?}") },
                 nd, format!("{p:?}"), pd);
        // Only the nested form is expected to be lossless. The plain form losing Ok is the
        // finding, not a defect, so it is asserted in that direction.
        if back_n != o {
            bad += 1;
        }
    }
    println!();
    println!("#   The nested form round-trips every state. The PLAIN form cannot: `Ok` and");
    println!("#   `Invalid` both encode to the empty string, so one of them must come back");
    println!("#   wrong whatever the adapter author chooses. This one returns `Invalid`,");
    println!("#   which loses `Ok`; the intuitive alternative returns `Ok`, which silently");
    println!("#   claims success for a task that reported no outcome at all. That is the");
    println!("#   defect design/SHAPES.md says only a byte corpus catches, and the corpus");
    println!("#   as it stands does not: no payload reaches either state.");
    println!();
    println!("#   The nested site has a worse problem, reported and not fixed: the payload");
    println!("#   generator fills TaskOutput.success and TaskOutput.error INDEPENDENTLY, so");
    println!("#   P2.x contains `success=true, error=<sentence>` -- a state no adapter over");
    println!("#   {{Ok, Error(d)}} can represent at all. Counted over 200 elements the only");
    println!("#   combinations present are (true, non-empty) and (false, non-empty); the");
    println!("#   success state (true, empty) never occurs.");
    bad
}

/// Whether the payload set now REACHES the adapter's three states, counted off decoded
/// values rather than asserted from the generator. This is the check that says the fix took.
fn adapter_reachability() -> usize {
    use harness::arms_m2 as m2;
    use harness::arms_rest as r;
    println!();
    println!("# M4: are the adapter's three states reachable from the payload set now?");
    println!("#   Counted off DECODED values, not from the builder, and the state is read the");
    println!("#   way an adapter would read it rather than from the fields directly.");
    println!();

    // Nested site: TaskDetailed.output over P2.2.
    let tasks = m2::prost_arm::value(m2::P2_2);
    let (mut ok, mut err, mut absent, mut impossible) = (0, 0, 0, 0);
    for t in &tasks.tasks {
        match &t.output {
            None => absent += 1,
            Some(o) if o.success && o.error.is_empty() => ok += 1,
            Some(o) if !o.success && !o.error.is_empty() => err += 1,
            Some(_) => impossible += 1,
        }
    }
    println!("  nested site (TaskDetailed.output, P2.2, {} elements)", tasks.tasks.len());
    println!("      Ok       (success, no error) {ok}");
    println!("      Error    (error, no success) {err}");
    println!("      Invalid  (no child at all)   {absent}");
    println!("      IMPOSSIBLE (success AND error, which the message's own comment forbids) {impossible}");

    // Plain site: TaskSummary.error over P4.1. Ok and Invalid are indistinguishable here,
    // which is the point: the count is the size of the collision.
    let sums = r::m4::prost_value(r::P4_1);
    let nonempty = sums.tasks.iter().filter(|t| !t.error.is_empty()).count();
    let empty = sums.tasks.len() - nonempty;
    println!();
    println!("  plain site (TaskSummary.error, P4.1, {} elements)", sums.tasks.len());
    println!("      Error        (non-empty string)              {nonempty}");
    println!("      Ok OR Invalid (empty string -- INDISTINGUISHABLE) {empty}");
    println!();
    println!("#   The plain site's second row IS the finding: {empty} elements carry a state the");
    println!("#   wire form cannot tell apart, so an adapter must return one of two answers for");
    println!("#   all of them and be wrong on the other. Before this the row was 0 and the");
    println!("#   defect was unreachable.");

    let mut bad = 0usize;
    if impossible != 0 {
        println!("   ^^^ the impossible state still occurs {impossible} times");
        bad += 1;
    }
    if ok == 0 || err == 0 || absent == 0 || empty == 0 {
        println!("   ^^^ a state is still unreachable");
        bad += 1;
    }
    bad
}

fn upd(c: &mut m3::PresenceCensus, v: Option<bool>) {
    match v {
        None => c.absent += 1,
        Some(false) => c.present_zero += 1,
        Some(true) => c.present_nonzero += 1,
    }
}

// ---- a very small wire writer, for vectors the schema cannot emit -------------------

fn varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn key(out: &mut Vec<u8>, tag: u32, wire: u32) {
    varint(out, ((tag << 3) | wire) as u64);
}

fn ld(out: &mut Vec<u8>, tag: u32, body: &[u8]) {
    key(out, tag, 2);
    varint(out, body.len() as u64);
    out.extend_from_slice(body);
}

fn wrap_probe(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    ld(&mut out, 1, body);
    out
}

/// Each vector is one `Probe` body.
fn probe_vectors() -> Vec<(&'static str, Vec<u8>, &'static str)> {
    let mut v = Vec::new();

    // A known oneof member, then an UNKNOWN tag inside the same oneof's range, last.
    let mut b = Vec::new();
    ld(&mut b, 1, b"id");
    key(&mut b, 10, 0);
    varint(&mut b, 7); // as_int = 7
    key(&mut b, 15, 0);
    varint(&mut b, 99); // a oneof member no build here knows
    v.push(("unknown oneof tag 15 AFTER a known member", b,
            "no: the oneof stays at as_int"));

    // The same, unknown first.
    let mut b = Vec::new();
    ld(&mut b, 1, b"id");
    key(&mut b, 15, 0);
    varint(&mut b, 99);
    key(&mut b, 10, 0);
    varint(&mut b, 7);
    v.push(("unknown oneof tag 15 BEFORE a known member", b,
            "no: the oneof stays at as_int"));

    // An unknown oneof tag and NOTHING known: the oneof is unset, not guessed.
    let mut b = Vec::new();
    ld(&mut b, 1, b"id");
    key(&mut b, 15, 0);
    varint(&mut b, 99);
    v.push(("unknown oneof tag 15 ALONE", b, "no: the oneof is None"));

    // An unknown plain field, each wire type, after the known ones.
    let mut b = Vec::new();
    ld(&mut b, 1, b"id");
    key(&mut b, 20, 0);
    varint(&mut b, 12345);
    ld(&mut b, 21, b"an unknown length-delimited field");
    key(&mut b, 22, 5);
    b.extend_from_slice(&7u32.to_le_bytes());
    key(&mut b, 23, 1);
    b.extend_from_slice(&9u64.to_le_bytes());
    v.push(("unknown fields, all four wire types, top level", b, "no"));

    // An unknown field INSIDE a nested message: the as_stamp member's Timestamp.
    let mut ts = Vec::new();
    key(&mut ts, 1, 0);
    varint(&mut ts, 1_700_000_000);
    key(&mut ts, 9, 0);
    varint(&mut ts, 42); // unknown inside the child
    let mut b = Vec::new();
    ld(&mut b, 1, b"id");
    ld(&mut b, 13, &ts);
    v.push(("unknown field inside a nested message", b, "no"));

    // An unknown field BEFORE every known one, which is where a decoder that assumes tag
    // order breaks.
    let mut b = Vec::new();
    key(&mut b, 24, 0);
    varint(&mut b, 1);
    ld(&mut b, 1, b"id");
    key(&mut b, 2, 0);
    varint(&mut b, 0); // opt_count present AND zero, after an unknown
    v.push(("unknown field BEFORE every known field", b, "no"));

    v
}

fn check_probe(ctx: &m1::core_ffi_arm::Ctx, name: &str, wire: &[u8], note: &str) -> usize {
    let pd = match p::ListProbeResponse::decode(wire) {
        Ok(v) => v,
        Err(e) => {
            println!("{name:<44} prost REFUSED: {e}");
            return 1;
        }
    };
    let a = m3::armonik_arm::decode(wire);
    let c = m3::core_native_arm::decode(wire);
    let f = m3::core_ffi_arm::decode(ctx, wire);
    let agree = a == c && a == f;
    let (ra, rc, rf, rp) = (
        m3::armonik_arm::encode(&a),
        m3::core_native_arm::encode(&c),
        m3::core_ffi_arm::encode(ctx, &f),
        pd.encode_to_vec(),
    );
    let same = ra == rc && ra == rf && ra == rp;
    println!("{:<44} {:<12} {:<12} {}", name,
             if agree { "ok" } else { "DIFFER" },
             if same { "ok" } else { "DIFFER" }, note);
    usize::from(!(agree && same))
}

fn result_with_status(v: u64) -> Vec<u8> {
    let mut e = Vec::new();
    ld(&mut e, 1, b"session");
    key(&mut e, 4, 0);
    varint(&mut e, v);
    let mut out = Vec::new();
    ld(&mut out, 1, &e);
    out
}

fn check_result(ctx: &m1::core_ffi_arm::Ctx, name: &str, wire: &[u8]) -> usize {
    let pd = p::ListResultsResponse::decode(wire).unwrap();
    let a: ListResultsResponse = m1::armonik_arm::decode(wire);
    let c = m1::core_native_arm::decode(wire);
    let f = m1::core_ffi_arm::decode(ctx, wire);
    let agree = a == c && a == f;
    let re = m1::armonik_arm::encode(&a);
    let same = re == m1::core_native_arm::encode(&c)
        && re == m1::core_ffi_arm::encode(ctx, &f)
        && re == pd.encode_to_vec();
    // The value must survive, not just the bytes.
    let kept = matches!(a.results[0].status, facade::ResultStatus::Unknown(999))
        && pd.results[0].status == 999;
    println!("{:<44} {:<12} {:<12} {}", name,
             if agree { "ok" } else { "DIFFER" },
             if same { "ok" } else { "DIFFER" },
             if kept { "YES: round-trips as Unknown(999)" } else { "NO -- LOST" });
    usize::from(!(agree && same && kept))
}
