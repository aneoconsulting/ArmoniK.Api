//! Boundary-call counts per payload per direction, from a counting build (README R5).
//!
//! Counted inside the core, at the entry points and at each function-pointer invocation, so
//! what is reported is what happened rather than what the shape of the code suggests. R5
//! also requires the boundary itself be shown to exist, which `gen/stage3.sh` does with
//! `nm -D --undefined-only`: a counting build alone is not evidence that a call happened,
//! because an inlined body carries its counters with it.
//!
//! The `class` column is per design/SHAPES.md: a message the real schema barely has is a
//! CONTROL and its row is not a verdict. It is labelled here and not only in SHAPES.md,
//! because reports get assembled from logs.

use ak_abi::*;
use harness::arms_m2;
use harness::arms::*;

fn main() {
    println!("# boundary-call counts, arm core-ffi-rust");
    println!("#   counting build: {}", if cfg!(feature = "count") { "YES" } else { "no (figures below are meaningless)" });
    println!("#   guard:          {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!();
    println!(
        "{:<7} {:<8} {:<9} {:>9} {:>8} {:>8} {:>10} {:>7} {:>16} {:>6}",
        "payload", "class", "direction", "elements", "forward", "reverse", "crossings", "transc", "prefix", "grows"
    );
    let mut regressions = 0usize;

    let c = core_ffi_arm::Ctx::new();

    // ---- M1, a real message: 3 scalars, 6 strings, 2 singular nested messages.
    for pid in [P1_1, P1_2, P1_3] {
        let v = core_ffi_arm::value(pid);
        let n = v.results.len();
        unsafe {
            let cold = core_ffi_arm::Ctx::new();
            core_ffi_arm::encode_into(&cold, &v);
            row(pid, "real", "enc cold", n, &take_enc(cold.enc));

            core_ffi_arm::encode_into(&c, &v);
            ak_enc_counters_reset(c.enc);
            core_ffi_arm::encode_into(&c, &v);
            row(pid, "real", "enc warm", n, &take_enc(c.enc));

            let b = core_ffi_arm::encode(&c, &v);
            core_ffi_arm::decode(&c, &b);
            ak_dec_counters_reset(c.dec);
            core_ffi_arm::decode(&c, &b);
            row(pid, "real", "decode", n, &take_dec(c.dec));
        }
    }

    // ---- M2, a real message, and the one the control plane actually moves.
    for pid in arms_m2::ALL {
        let v = arms_m2::core_ffi_arm::value(pid);
        let n = v.tasks.len();
        unsafe {
            let cold = core_ffi_arm::Ctx::new();
            arms_m2::core_ffi_arm::encode_into(&cold, &v);
            row(pid, "real", "enc cold", n, &take_enc(cold.enc));

            arms_m2::core_ffi_arm::encode_into(&c, &v);
            ak_enc_counters_reset(c.enc);
            arms_m2::core_ffi_arm::encode_into(&c, &v);
            row(pid, "real", "enc warm", n, &take_enc(c.enc));

            let b = arms_m2::core_ffi_arm::encode(&c, &v);
            arms_m2::core_ffi_arm::decode(&c, &b);
            ak_dec_counters_reset(c.dec);
            arms_m2::core_ffi_arm::decode(&c, &b);
            row(pid, "real", "decode", n, &take_dec(c.dec));
        }
    }

    // ---- M3: a oneof and explicit presence. Both ride in the group, so neither adds a
    // crossing; that is the point and the count is how it is shown rather than asserted.
    {
        use harness::arms_m3 as m3;
        let v = m3::core_ffi_arm::value(m3::P3_1);
        let n = v.probes.len();
        unsafe {
            let cold = core_ffi_arm::Ctx::new();
            m3::core_ffi_arm::encode_into(&cold, &v);
            row("P3.1", "real", "enc cold", n, &take_enc(cold.enc));
            m3::core_ffi_arm::encode_into(&c, &v);
            ak_enc_counters_reset(c.enc);
            m3::core_ffi_arm::encode_into(&c, &v);
            row("P3.1", "real", "enc warm", n, &take_enc(c.enc));
            let b = m3::core_ffi_arm::encode(&c, &v);
            m3::core_ffi_arm::decode(&c, &b);
            ak_dec_counters_reset(c.dec);
            m3::core_ffi_arm::decode(&c, &b);
            row("P3.1", "real", "decode", n, &take_dec(c.dec));
        }
    }

    // ---- ABI v1 open decision 5, by site. An aggregate that does not say WHERE the
    // learned width thrashes cannot answer what the grow path costs.
    println!();
    println!("# length-prefix misses by site, warm context (ABI v1 open decision 5)");
    println!("#   a site is one length-prefix call site in the generated codec. Its learned");
    println!("#   width is per context and starts at one byte.");
    println!();
    for pid in ["P1.2", "P2.2", "P2.4"] {
        let ctx = core_ffi_arm::Ctx::new();
        unsafe {
            if pid == "P1.2" {
                let v = core_ffi_arm::value(pid);
                core_ffi_arm::encode_into(&ctx, &v);
                ak_enc_counters_reset(ctx.enc);
                let mut z = vec![0u32; SITE_NAMES.len()];
                ak_enc_site_moves(ctx.enc, z.as_mut_ptr(), z.len());
                let before = z.clone();
                core_ffi_arm::encode_into(&ctx, &v);
                ak_enc_site_moves(ctx.enc, z.as_mut_ptr(), z.len());
                site_report(pid, v.results.len(), &before, &z);
            } else {
                let v = arms_m2::core_ffi_arm::value(pid);
                arms_m2::core_ffi_arm::encode_into(&ctx, &v);
                let mut z = vec![0u32; SITE_NAMES.len()];
                ak_enc_site_moves(ctx.enc, z.as_mut_ptr(), z.len());
                let before = z.clone();
                arms_m2::core_ffi_arm::encode_into(&ctx, &v);
                ak_enc_site_moves(ctx.enc, z.as_mut_ptr(), z.len());
                site_report(pid, v.tasks.len(), &before, &z);
            }
        }
    }

    // A regression that would have caught the open-state leak: on a payload whose elements
    // are all the same shape, a learned width converges after the first pass and a warm
    // context must never miss again. It missed 448 times in 500 elements because the second
    // and later chunks of an element run read whatever the previous element left in the
    // codec's open-field state.
    println!();
    println!("# regression: a warm context must not miss on a UNIFORM payload");
    for pid in ["P1.2", "P2.2", "P2.3", "P2.5"] {
        let ctx = core_ffi_arm::Ctx::new();
        let misses = unsafe {
            if pid == "P1.2" {
                let v = core_ffi_arm::value(pid);
                core_ffi_arm::encode_into(&ctx, &v);
                ak_enc_counters_reset(ctx.enc);
                core_ffi_arm::encode_into(&ctx, &v);
            } else {
                let v = arms_m2::core_ffi_arm::value(pid);
                arms_m2::core_ffi_arm::encode_into(&ctx, &v);
                ak_enc_counters_reset(ctx.enc);
                arms_m2::core_ffi_arm::encode_into(&ctx, &v);
            }
            take_enc(ctx.enc).prefix_moves
        };
        println!("#   {pid:<6} warm misses = {misses} {}", if misses == 0 { "ok" } else { "REGRESSION" });
        if misses != 0 {
            regressions += 1;
        }
    }
    if regressions > 0 {
        println!();
        println!("FAILED: {regressions} uniform payload(s) do not converge.");
        std::process::exit(1);
    }

    println!();
    println!("# `crossings` is forward + reverse: calls that leave one side's compilation unit.");
    println!("# `transc` counts transcoder invocations, which are indirect calls the core makes");
    println!("#   into ITSELF (ABI v1 section 4 puts every representation in the core), so they");
    println!("#   are not host crossings. Counting them together is what made the drafted");
    println!("#   interface look like 15,137 calls on a 1,000-row response.");
    println!("# `prefix` counts length-placeholder resizes: ABI v1 open decision 5. `enc cold`");
    println!("#   is a context whose learned widths have never been set, `enc warm` the steady");
    println!("#   state. P2.4 alternates element bodies across a varint length boundary by");
    println!("#   construction, so a per-site learned width is wrong on every element of it:");
    println!("#   that row is the one decision 5 turns on.");
    println!("# M1's element type is a LEAF, so its rows batch. M2's is not (ABI v1 7.2), so");
    println!("#   M2 costs two calls per element plus one run per repeated field it carries.");
}

fn site_report(pid: &str, elems: usize, before: &[u32], after: &[u32]) {
    let mut rows: Vec<(u32, &str)> = SITE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| (after[i].saturating_sub(before[i]), *n))
        .filter(|(d, _)| *d > 0)
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    let total: u32 = rows.iter().map(|(d, _)| d).sum();
    if rows.is_empty() {
        println!("  {pid:<6} {elems} elements: no misses in the warm pass");
        return;
    }
    println!("  {pid:<6} {elems} elements, {total} misses in one warm pass:");
    for (d, n) in rows {
        println!("      {d:>6}  {:>7}  {n}", format!("{:.2}/el", d as f64 / elems as f64));
    }
}

unsafe fn take_enc(ctx: *mut ak_enc_ctx) -> AkCounters {
    let mut z = AkCounters::default();
    ak_enc_counters(ctx, &mut z);
    z
}

unsafe fn take_dec(ctx: *mut ak_dec_ctx) -> AkCounters {
    let mut z = AkCounters::default();
    ak_dec_counters(ctx, &mut z);
    z
}

fn row(pid: &str, class: &str, dir: &str, elems: usize, c: &AkCounters) {
    let cr = c.forward + c.reverse;
    println!(
        "{:<7} {:<8} {:<9} {:>9} {:>8} {:>8} {:>10} {:>7} {:>16} {:>6}",
        pid, class, dir, elems, c.forward, c.reverse, cr, c.transcode,
        if c.prefix_moves == 0 { "0".to_string() } else { format!("{} ({} B)", c.prefix_moves, c.prefix_bytes) },
        c.grows
    );
    if elems > 0 && dir != "enc cold" {
        println!(
            "{:<7} {:<8} {:<9} {:>9} {:>8} {:>8} {:>10} {:>7} {:>16} {:>6}",
            "", "", "  per elem", "", "", "", format!("{:.3}", cr as f64 / elems as f64), "", "", ""
        );
    }
}
