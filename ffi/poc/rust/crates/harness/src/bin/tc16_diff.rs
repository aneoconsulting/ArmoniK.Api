//! D19's differential: the simdutf transcoder (`ak_tc_utf16`) against the pre-D19 scalar
//! one (`ak_tc_utf16_scalar`, the oracle) and std's `String::from_utf16_lossy` (an oracle
//! that is neither), and the additive exports against std and the old functions. Nothing
//! timed. Exit 1 on any difference; the planted controls must each produce differences.
//!
//!   A  transcoders: every content set x every length 0..=100, then sampled lengths up to
//!      4 Ki and a few large ones, x seven capacity / grow cases (ample; exactly 3n; zero
//!      with the core's geometric grow; exactly the output; one short with a grow limited
//!      to the output, and to one less; the output with a refusing grow; one short with a
//!      refusing grow). Compared: return code, bytes, and a 64-byte canary past the last
//!      capacity handed out.
//!   B  the exports: ak_utf16_to_utf8 (caps 3n, exact, exact-1, 0), ak_utf16_utf8_len,
//!      ak_utf16_validate; ak_utf8_to_utf16 / ak_utf8_utf16_len / ak_utf8_validate on the
//!      valid outputs of A and on invalid UTF-8 (random bytes, mutated valid strings, the
//!      RFC 3629 edge cases), against std and ak_utf8_check.
//!   C  through a real encode context (the core's `ak_grow` and `enc_blob`): ListResultsResponse
//!      with every string field delivered as UTF-16 through ak_tc_utf16, through
//!      ak_tc_utf16_scalar, and prost over the lossy strings. Fresh context per message (grows
//!      from the initial buffer) and one reused context.
//!   P  planted controls: a `?`-substituting transcoder in A's slot (must differ on every
//!      set with a lone surrogate and nowhere else), an overrunning one (the canary must
//!      catch it).
//!
//! `AK_D19_SCALE` (default 1) multiplies the per-length repetitions.
use ak_abi::*;
use core::ffi::c_void;
use harness::d19::*;
use std::time::Instant;

struct Tally {
    checks: u64,
    fails: u64,
    shown: u32,
}
impl Tally {
    fn check(&mut self, ok: bool, what: impl FnOnce() -> String) {
        self.checks += 1;
        if !ok {
            self.fails += 1;
            if self.shown < 20 {
                self.shown += 1;
                println!("  FAIL {}", what());
            }
        }
    }
}

fn lengths(r: &mut Rng, scale: usize) -> Vec<(usize, usize)> {
    // (length, repetitions)
    let mut v: Vec<(usize, usize)> = (0..=100).map(|n| (n, 400 * scale)).collect();
    for _ in 0..300 {
        v.push((101 + r.below(4096 - 101) as usize, 20 * scale));
    }
    for n in [127, 128, 255, 256, 511, 512, 1023, 1024, 2047, 2048, 4095, 4096, 4097, 5461, 5462, 16383, 16384] {
        v.push((n, 20 * scale));
    }
    for n in [65_536, 65_537, 1 << 20] {
        v.push((n, 2));
    }
    v
}

fn cases(want: usize, n: usize) -> Vec<(&'static str, usize, Grow)> {
    vec![
        ("ample", 3 * n + 16, Grow::Geometric),
        ("cap=3n", 3 * n, Grow::Geometric),
        ("cap=0 geometric", 0, Grow::Geometric),
        ("cap=out", want, Grow::Geometric),
        ("cap=out-1 grow<=out", want.saturating_sub(1), Grow::Limit(want)),
        ("cap=out-1 grow<=out-1", want.saturating_sub(1), Grow::Limit(want.saturating_sub(1))),
        ("cap=out refusing grow", want, Grow::Refuse),
        ("cap=out-1 refusing grow", want.saturating_sub(1), Grow::Refuse),
    ]
}

fn main() {
    let scale: usize = std::env::var("AK_D19_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let seed: u64 = std::env::var("AK_D19_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(0xD19_5EED);
    let _init = harness::arms::core_ffi_arm::Ctx::new();
    let new_tc = unsafe { ak_tc_utf16() };
    let old_tc = unsafe { ak_tc_utf16_scalar() };
    println!("D19 differential: seed {seed:#x}, scale {scale}");
    println!("ak_tc_utf16 {:p}, ak_tc_utf16_scalar {:p} (must differ)", new_tc as *const (), old_tc as *const ());
    assert_ne!(new_tc as usize, old_tc as usize);
    let base = base_core_tc16();
    match &base {
        Some((p, f)) => println!("pre-D19 core: {p}, its ak_tc_utf16 {:p} (compared in A against the scalar export: bytes, return and grow requests)", *f as *const ()),
        None => println!("pre-D19 core: not loaded (AK_D19_BASE_CORE unset)"),
    }
    let mut base_checks = 0u64;
    let t0 = Instant::now();

    // ---------------------------------------------------------------- A and B
    let mut a = Tally { checks: 0, fails: 0, shown: 0 };
    let mut b = Tally { checks: 0, fails: 0, shown: 0 };
    let mut plant_q = [0u64; 8]; // per set: inputs where the planted `?` transcoder differs
    let mut inputs = [0u64; 8];
    let mut invalid_inputs = [0u64; 8];
    let mut invalid_small = [0u64; 8]; // invalid inputs of at most 4 Ki units (the plant's range)
    let mut bytes_in = 0u64;
    let mut r = Rng(seed);
    let lens = lengths(&mut r, scale);
    let mut total_inputs = 0u64;
    let mut new_grows = 0u64;
    let mut old_grows = 0u64;
    for (si, &set) in ALL_SETS.iter().enumerate() {
        for &(n, reps) in &lens {
            for _ in 0..reps {
                let u = gen(&mut r, set, n);
                total_inputs += 1;
                inputs[si] += 1;
                bytes_in += 2 * n as u64;
                let lossy = String::from_utf16_lossy(&u).into_bytes();
                let valid = char::decode_utf16(u.iter().copied()).all(|c| c.is_ok());
                if !valid {
                    invalid_inputs[si] += 1;
                    if n <= 4096 {
                        invalid_small[si] += 1;
                    }
                }
                let want = lossy.len();
                // A: every capacity / grow case, old and new, and std.
                for (name, cap, mode) in cases(want, n) {
                    let (o_new, s_new) = unsafe { run_tc(new_tc, &u, cap, mode) };
                    let (o_old, s_old) = unsafe { run_tc(old_tc, &u, cap, mode) };
                    new_grows += s_new.grows as u64;
                    old_grows += s_old.grows as u64;
                    if let Some((_, base_tc)) = &base {
                        // The pre-D19 core's own binary: the scalar export is that code.
                        let (o_base, s_base) = unsafe { run_tc(*base_tc, &u, cap, mode) };
                        base_checks += 1;
                        a.check(o_base == o_old && s_base.wants == s_old.wants, || {
                            format!("A {} n={} {}: pre-D19 core rc {} grows {:?}, scalar export rc {} grows {:?}", set.name(), n, name, o_base.rc, s_base.wants, o_old.rc, s_old.wants)
                        });
                    }
                    a.check(o_new == o_old, || {
                        format!("A {} n={} {}: new rc {} ({} B, canary {}) old rc {} ({} B, canary {}) u[..8]={:04x?}",
                            set.name(), n, name, o_new.rc, o_new.bytes.len(), o_new.canary_ok, o_old.rc, o_old.bytes.len(), o_old.canary_ok, &u[..u.len().min(8)])
                    });
                    a.check(o_new.canary_ok, || format!("A {} n={} {}: new transcoder wrote past its capacity", set.name(), n, name));
                    if o_new.rc >= 0 {
                        a.check(o_new.bytes == lossy, || format!("A {} n={} {}: new != from_utf16_lossy", set.name(), n, name));
                    } else {
                        // Only a capacity the grow could not raise to the output may fail.
                        let may_fail = cap < want
                            && match mode {
                                Grow::Refuse => true,
                                Grow::Limit(l) => l < want,
                                Grow::Geometric => false,
                            };
                        a.check(may_fail, || format!("A {} n={} {}: new refused (rc {}) where the output fits", set.name(), n, name, o_new.rc));
                    }
                }
                // The planted `?` transcoder in the new transcoder's slot, ample case.
                if n <= 4096 {
                    let (o_p, _) = unsafe { run_tc(planted_question_mark, &u, 3 * n + 16, Grow::Geometric) };
                    let (o_old, _) = unsafe { run_tc(old_tc, &u, 3 * n + 16, Grow::Geometric) };
                    if o_p != o_old {
                        plant_q[si] += 1;
                    }
                }
                // B: the UTF-16 exports.
                unsafe {
                    let mut d = vec![CANARY_BYTE; 3 * n + CANARY];
                    for cap in [3 * n, want, want.saturating_sub(1), 0] {
                        d.iter_mut().for_each(|x| *x = CANARY_BYTE);
                        let src = if u.is_empty() { core::ptr::null() } else { u.as_ptr() };
                        let rc = ak_utf16_to_utf8(src, n, d.as_mut_ptr(), cap);
                        let canary = d[cap..].iter().all(|&x| x == CANARY_BYTE);
                        b.check(canary, || format!("B ak_utf16_to_utf8 {} n={} cap={}: wrote past cap", set.name(), n, cap));
                        if cap >= want {
                            b.check(rc == want as i32 && d[..want] == lossy[..], || {
                                format!("B ak_utf16_to_utf8 {} n={} cap={}: rc {} want {}", set.name(), n, cap, rc, want)
                            });
                        } else {
                            b.check(rc == AK_ERR_CAPACITY, || format!("B ak_utf16_to_utf8 {} n={} cap={}: rc {} want CAPACITY", set.name(), n, cap, rc));
                        }
                    }
                    let src = if u.is_empty() { core::ptr::null() } else { u.as_ptr() };
                    let l = ak_utf16_utf8_len(src, n);
                    b.check(l == want as i32, || format!("B ak_utf16_utf8_len {} n={}: {} want {}", set.name(), n, l, want));
                    let v = ak_utf16_validate(src, n);
                    b.check((v == AK_OK) == valid && (v == AK_OK || v == AK_ERR_TRANSCODE), || format!("B ak_utf16_validate {} n={}: {} valid {}", set.name(), n, v, valid));
                    // UTF-8 -> UTF-16 on the (valid) lossy output: std's encode_utf16.
                    if n <= 4096 || r.below(4) == 0 {
                        check_utf8_exports(&mut b, &lossy, set.name());
                        // And on a mutated copy: one random byte replaced.
                        if !lossy.is_empty() {
                            let mut m = lossy.clone();
                            let at = r.below(m.len() as u64) as usize;
                            m[at] = r.next() as u8;
                            check_utf8_exports(&mut b, &m, "mutated");
                        }
                    }
                }
            }
        }
    }
    // B: invalid UTF-8 the RFC 3629 way, and random bytes.
    let edge: [&[u8]; 16] = [
        b"\xC0\x80", b"\xC1\xBF", b"\xE0\x80\x80", b"\xE0\x9F\xBF", b"\xED\xA0\x80", b"\xED\xBF\xBF", b"\xF0\x80\x80\x80",
        b"\xF0\x8F\xBF\xBF", b"\xF4\x90\x80\x80", b"\xF5\x80\x80\x80", b"\xFF", b"\x80", b"\xE2\x82", b"a\xF0\x9F\x98",
        b"\xEF\xBF\xBD\xED\xB0\x80", b"\xF4\x8F\xBF\xBF",
    ];
    for e in edge {
        for pre in 0..40usize {
            let mut m = vec![b'x'; pre];
            m.extend_from_slice(e);
            unsafe { check_utf8_exports(&mut b, &m, "rfc3629-edge") };
        }
    }
    let mut rr = Rng(seed ^ 0xBADC0DE);
    for _ in 0..200_000 * scale {
        let n = rr.below(200) as usize;
        let m: Vec<u8> = (0..n).map(|_| rr.next() as u8).collect();
        unsafe { check_utf8_exports(&mut b, &m, "random-bytes") };
    }
    let t_ab = t0.elapsed();

    // ---------------------------------------------------------------- C
    let mut c = Tally { checks: 0, fails: 0, shown: 0 };
    let mut rc_ = Rng(seed ^ 0xC0FFEE);
    let reused = unsafe { ak_enc_ctx_new() };
    let mut msgs = 0u64;
    let mut c_strings = 0u64;
    for i in 0..20_000 * scale {
        let nres = rc_.below(24) as usize;
        let mut res = Vec::with_capacity(nres);
        for _ in 0..nres {
            let mut f = Vec::with_capacity(5);
            for _ in 0..5 {
                let set = ALL_SETS[rc_.below(ALL_SETS.len() as u64) as usize];
                let n = match rc_.below(20) {
                    0 => 5000 + rc_.below(20_000) as usize, // 2- and 3-byte length prefixes
                    1..=3 => 0,
                    _ => rc_.below(200) as usize,
                };
                f.push(gen(&mut rc_, set, n));
                c_strings += 1;
            }
            res.push((f, (0..rc_.below(20)).map(|_| rc_.next() as u8).collect::<Vec<u8>>(), rc_.next() as i64));
        }
        let prost_bytes = {
            use shapes_prost::shapes::*;
            let m = ListResultsResponse {
                results: res
                    .iter()
                    .map(|(f, opaque, size)| ResultRaw {
                        session_id: String::from_utf16_lossy(&f[0]),
                        name: String::from_utf16_lossy(&f[1]),
                        owner_task_id: String::from_utf16_lossy(&f[2]),
                        result_id: String::from_utf16_lossy(&f[3]),
                        created_by: String::from_utf16_lossy(&f[4]),
                        opaque_id: opaque.clone(),
                        size: *size,
                        ..Default::default()
                    })
                    .collect(),
                page: i as i32,
                total: nres as i32,
            };
            prost::Message::encode_to_vec(&m)
        };
        for (label, tc) in [("new", new_tc), ("scalar", old_tc)] {
            for fresh in [true, false] {
                let ctx = if fresh { unsafe { ak_enc_ctx_new() } } else { reused };
                let got = unsafe { encode_m1(ctx, &res, i as i32, tc) };
                if fresh {
                    unsafe { ak_enc_ctx_free(ctx) };
                }
                c.check(got.as_deref() == Ok(&prost_bytes[..]), || {
                    format!("C msg {i} {label} fresh={fresh}: {:?} vs prost {} B", got.as_ref().map(|v| v.len()), prost_bytes.len())
                });
            }
        }
        msgs += 1;
    }
    unsafe { ak_enc_ctx_free(reused) };
    let t_c = t0.elapsed() - t_ab;

    // ---------------------------------------------------------------- P: the overrun plant
    let mut over_caught = 0u64;
    let mut over_runs = 0u64;
    let mut rp = Rng(seed ^ 0x0E4);
    for _ in 0..1000 {
        let n = 1 + rp.below(100) as usize;
        let u = gen(&mut rp, Set::Mixed, n);
        let want = String::from_utf16_lossy(&u).len();
        let (o, _) = unsafe { run_tc(planted_overrun, &u, want, Grow::Geometric) };
        over_runs += 1;
        if !o.canary_ok {
            over_caught += 1;
        }
    }

    // ---------------------------------------------------------------- report
    println!();
    println!("A  transcoders: {total_inputs} inputs ({:.1} MB of UTF-16), {} checks, {} failures; grow calls over all cases: new {new_grows}, scalar {old_grows}", bytes_in as f64 / 1e6, a.checks, a.fails);
    println!("   per set: inputs / with a lone surrogate / of those, up to 4 Ki units / where the planted `?` transcoder differs from the oracle (up to 4 Ki units):");
    for (si, set) in ALL_SETS.iter().enumerate() {
        println!("     {:<16} {:>9} {:>9} {:>9} {:>9}", set.name(), inputs[si], invalid_inputs[si], invalid_small[si], plant_q[si]);
    }
    println!("   of A's checks, {base_checks} compare the pre-D19 core's binary with the scalar export");
    println!("B  exports: {} checks, {} failures", b.checks, b.fails);
    println!("C  encode context, ListResultsResponse, 5 UTF-16 string fields per element: {msgs} messages, {c_strings} strings, x {{new, scalar}} x {{fresh, reused}} vs prost: {} checks, {} failures", c.checks, c.fails);
    println!("P  overrun plant: caught {over_caught} of {over_runs}");
    println!("time: A+B {:.1} s, C {:.1} s", t_ab.as_secs_f64(), t_c.as_secs_f64());
    // The `?` plant must differ exactly where a lone surrogate is: on every invalid input in
    // its range and on nothing else (so never on a valid set, and on each invalid set).
    let mut plant_ok = true;
    for (si, set) in ALL_SETS.iter().enumerate() {
        let valid_set = VALID_SETS.contains(set);
        if plant_q[si] != invalid_small[si] || (valid_set && plant_q[si] != 0) || (!valid_set && plant_q[si] == 0) {
            plant_ok = false;
        }
    }
    let ok = a.fails == 0 && b.fails == 0 && c.fails == 0 && plant_ok && over_caught == over_runs;
    println!("planted `?` transcoder: {}", if plant_ok { "differs on every invalid set and on no valid set (caught)" } else { "NOT CAUGHT as required" });
    println!("{}", if ok { "D19 DIFFERENTIAL PASSED" } else { "D19 DIFFERENTIAL FAILED" });
    std::process::exit(if ok { 0 } else { 1 });
}

unsafe fn check_utf8_exports(t: &mut Tally, m: &[u8], what: &str) {
    let n = m.len();
    let std_ok = std::str::from_utf8(m).is_ok();
    let src = if m.is_empty() { core::ptr::null() } else { m.as_ptr() };
    let v = ak_utf8_validate(src, n);
    let chk = ak_utf8_check(src, n);
    t.check((v == AK_OK) == std_ok && (v == AK_OK || v == AK_ERR_TRANSCODE), || format!("B ak_utf8_validate {what} n={n}: {v} std {std_ok}"));
    t.check(v == chk, || format!("B ak_utf8_validate {what} n={n}: {v} ak_utf8_check {chk}"));
    let want: Option<Vec<u16>> = std::str::from_utf8(m).ok().map(|s| s.encode_utf16().collect());
    let mut d = vec![0x5A5Au16; n + CANARY];
    for cap in [n, want.as_ref().map_or(n, |w| w.len()), want.as_ref().map_or(0, |w| w.len().saturating_sub(1)), 0] {
        d.iter_mut().for_each(|x| *x = 0x5A5A);
        let rc = ak_utf8_to_utf16(src, n, d.as_mut_ptr(), cap);
        let canary = d[cap..].iter().all(|&x| x == 0x5A5A);
        t.check(canary, || format!("B ak_utf8_to_utf16 {what} n={n} cap={cap}: wrote past cap"));
        match &want {
            Some(w) if cap >= w.len() => {
                t.check(rc == w.len() as i32 && d[..w.len()] == w[..], || format!("B ak_utf8_to_utf16 {what} n={n} cap={cap}: rc {rc} want {}", w.len()));
            }
            Some(_) => t.check(rc == AK_ERR_CAPACITY, || format!("B ak_utf8_to_utf16 {what} n={n} cap={cap}: rc {rc} want CAPACITY")),
            None => t.check(rc == AK_ERR_TRANSCODE, || format!("B ak_utf8_to_utf16 {what} n={n} cap={cap}: rc {rc} want TRANSCODE")),
        }
    }
    if let Some(w) = &want {
        let l = ak_utf8_utf16_len(src, n);
        t.check(l == w.len() as i32, || format!("B ak_utf8_utf16_len {what} n={n}: {l} want {}", w.len()));
    }
}

/// ListResultsResponse through the C ABI with every string field of every element as UTF-16
/// through `tc`, `opaque_id` through ak_tc_bytes. Reset first; the bytes after ak_enc_take.
unsafe fn encode_m1(ctx: *mut ak_enc_ctx, res: &[(Vec<Vec<u16>>, Vec<u8>, i64)], page: i32, tc: ak_transcode_fn) -> Result<Vec<u8>, i32> {
    struct Obj<'a> {
        res: &'a [(Vec<Vec<u16>>, Vec<u8>, i64)],
        tc: ak_transcode_fn,
    }
    unsafe extern "C" fn loop_results(ctx: *mut ak_enc_ctx, obj: *const c_void, _tok: i64) -> i32 {
        let o = &*(obj as *const Obj);
        let s16 = |u: &Vec<u16>| ak_str {
            data: if u.is_empty() { core::ptr::null() } else { u.as_ptr() as *const c_void },
            len: u.len(),
            tc: Some(o.tc),
        };
        let g: Vec<ak_efix_ResultRaw> = o
            .res
            .iter()
            .map(|(f, opaque, size)| ak_efix_ResultRaw {
                session_id: s16(&f[0]),
                name: s16(&f[1]),
                owner_task_id: s16(&f[2]),
                result_id: s16(&f[3]),
                created_by: s16(&f[4]),
                opaque_id: ak_str {
                    data: if opaque.is_empty() { core::ptr::null() } else { opaque.as_ptr() as *const c_void },
                    len: opaque.len(),
                    tc: Some(ak_tc_bytes()),
                },
                size: *size,
                ..ak_efix_ResultRaw::ZERO
            })
            .collect();
        if g.is_empty() {
            return AK_OK;
        }
        ak_elem_ResultRaw(ctx, g.as_ptr(), g.len() as i32)
    }
    let obj = Obj { res, tc };
    ak_enc_reset(ctx);
    let vt = ak_evt_ListResultsResponse { loop_results: Some(loop_results) };
    let fix = ak_efix_ListResultsResponse { page, total: res.len() as i32, presence: 0 };
    let rc = ak_encode_ListResultsResponse(&obj as *const Obj as *const c_void, ctx, &vt, &fix);
    if rc < 0 {
        return Err(rc as i32);
    }
    let (mut p, mut n) = (core::ptr::null::<u8>(), 0usize);
    let rc = ak_enc_take(ctx, &mut p, &mut n);
    if rc != AK_OK {
        return Err(rc);
    }
    Ok(if n == 0 { Vec::new() } else { core::slice::from_raw_parts(p, n).to_vec() })
}
