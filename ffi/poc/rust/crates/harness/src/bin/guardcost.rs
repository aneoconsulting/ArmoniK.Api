//! What ABI v1 section 3's "every entry point requires `ak_init`" costs, per crossing.
//!
//! **Why this binary exists, and it is a method finding as much as a measurement.**
//! The obvious way to price the guard is two builds -- `--features init-guard` on and off --
//! and a benchmark in each.  That was tried first (`gen/guardprice.sh`, four `bench` runs,
//! in `ffi/logs/rust/stage5-lifecycle.log`) and **it does not survive its own control**.
//! R4 requires a cross-build comparison to carry an in-process control, and the control
//! here is `core-native`, which has no guard in either build and therefore must not move.
//! It moved by up to 30 percent.  A ratio whose control moved is not a figure, so no guard
//! price is quoted from that run.
//!
//! This is the same question asked the way R4's sharpened half says to ask it: **a delta
//! between two arms in the same interleaved rounds of one process and one build**.
//! `ak_noop` is a bare crossing; `ak_noop_guarded` is the same crossing with the guard the
//! generator emits into every entry point -- one relaxed load of the process-global init
//! word, a compare, a branch.  The difference is the guard and nothing else.
//!
//! **What it gives, and what it does not.**  It gives a per-CROSSING price.  The per-payload
//! cost is that times the forward-crossing count, which the counting build already reports
//! and which is a property of the descriptor rather than of this machine: 1 per decode on
//! every payload, and on encode 2 (P1.1) to 2,511 (P2.2).  It does NOT give what the guard
//! costs inside a real entry point, where the branch may schedule differently among the
//! work already there -- that would need the two-build form, and the two-build form is what
//! did not hold.

use ak_abi::*;
use std::time::Instant;

const ROUNDS: usize = 21;
const REPS: usize = 2_000_000;

fn main() {
    println!("# ABI v1 section 3's guard, priced as a within-process delta");
    println!("#   arms: ak_noop (bare crossing) and ak_noop_guarded (the same plus the");
    println!("#         guard the generator emits into every entry point)");
    println!("#   one process, one build, arms interleaved round-robin (R3, R4)");
    println!("#   rounds: {ROUNDS}, {REPS} crossings each");
    println!();

    // Reaching the harness lib is what pulls the cdylib into the link: this binary calls
    // only `extern "C"` symbols otherwise, and the linker drops a library nothing in the
    // Rust crate graph references. It doubles as the check every other arm makes.
    assert_eq!(harness::abi_version(), AK_ABI_VERSION);

    // The guard is only cheap when it PASSES, which is the state every real call is in, so
    // the library is initialised first. A guard measured in the failing state would be
    // measuring the branch predictor's easy case in the direction nobody runs.
    unsafe {
        let o = ak_init_opts {
            abi_version: AK_ABI_VERSION,
            flags: AK_INIT_NO_CRYPTO | AK_INIT_NO_PANIC_HOOK,
            log: None,
            log_ctx: std::ptr::null_mut(),
        };
        let mut e = ak_err::default();
        let rc = ak_init(&o, &mut e);
        assert!(rc >= 0, "ak_init failed: {rc}");
        assert_eq!(ak_initialized(), 1);
        // And the guarded arm must be returning the REAL answer, not the refusal: if it
        // were refusing, this would be a benchmark of an early return.
        assert_eq!(ak_noop_guarded(7), ak_noop(7), "the guarded arm is refusing");
        assert_eq!(ak_noop2(7), ak_noop(7), "the control arm is not the same function");
    }

    let mut bare: Vec<f64> = Vec::new();
    let mut bare2: Vec<f64> = Vec::new();
    let mut guarded: Vec<f64> = Vec::new();
    // Warm all three, outside the clock.
    black(run_bare(100_000));
    black(run_bare2(100_000));
    black(run_guarded(100_000));

    for _ in 0..ROUNDS {
        let t = Instant::now();
        black(run_bare(REPS));
        bare.push(t.elapsed().as_nanos() as f64 / REPS as f64);
        let t = Instant::now();
        black(run_bare2(REPS));
        bare2.push(t.elapsed().as_nanos() as f64 / REPS as f64);
        let t = Instant::now();
        black(run_guarded(REPS));
        guarded.push(t.elapsed().as_nanos() as f64 / REPS as f64);
    }

    let (bm, blo, bhi) = stats(&bare);
    let (b2m, b2lo, b2hi) = stats(&bare2);
    let (gm, glo, ghi) = stats(&guarded);
    println!("{:<22} {:>12} {:>12} {:>12}", "arm", "median ns", "min ns", "max ns");
    println!("{:<22} {:>12.4} {:>12.4} {:>12.4}", "ak_noop  (twin A)", bm, blo, bhi);
    println!("{:<22} {:>12.4} {:>12.4} {:>12.4}", "ak_noop2 (twin B)", b2m, b2lo, b2hi);
    println!("{:<22} {:>12.4} {:>12.4} {:>12.4}", "ak_noop_guarded", gm, glo, ghi);
    println!();

    // The two twins are byte-for-byte the same function with no guard in either, so their
    // disagreement is the FLOOR: it is what two exported symbols cost each other through
    // address, cache line, alignment and PLT slot, and nothing else.
    let floor = (b2m - bm).abs();
    // The guarded arm is compared with BOTH twins, because picking the nearer one after
    // the fact would be choosing the answer.
    let (va, vb) = (gm - bm, gm - b2m);

    println!("{:<34} {:>+12.4}", "twin B - twin A   (the FLOOR)", b2m - bm);
    println!("{:<34} {:>+12.4}", "guarded - twin A", va);
    println!("{:<34} {:>+12.4}", "guarded - twin B", vb);
    println!();
    println!("# THE FLOOR IS THE VERDICT, AND IT IS WHY THE TWIN IS HERE. Two exported");
    println!("# functions with identical bodies and no guard in either differ by {:+.4} ns.", b2m - bm);
    println!("# At a crossing of about {:.1} ns that is not small, and it is not stable across", bm.min(b2m));
    println!("# builds either: an earlier build of this binary, WITHOUT the twin, put the");
    println!("# penalty on the other function and made the guard look like it cost 0.70 ns.");
    println!("# It is layout, and the twin is how that is shown rather than argued.");
    println!();
    let worst = va.abs().max(vb.abs());
    let best = va.abs().min(vb.abs());
    if best <= floor * 1.5 {
        println!("# The guarded arm lands within {:+.4} ns of one twin and {:+.4} ns of the", best, worst);
        println!("# other, and the twins are {:.4} ns apart on their own. **The guard is under", floor);
        println!("# this method's floor**, so the honest statement is a BOUND and not a value:");
        println!("#");
        println!("#     |guard| < {:.2} ns per crossing, and plausibly ~0", floor.max(worst));
        println!("#");
        println!("# A value would be the wrong claim in both directions here: a NEGATIVE one");
        println!("# would say a load and a branch make a call cheaper, which they cannot, and");
        println!("# a positive one taken against the unlucky twin would be reporting the");
        println!("# linker's choices as the specification's cost.");
    } else {
        println!("# The guarded arm is {:+.4} and {:+.4} ns from the two twins, both outside", va, vb);
        println!("# the {:.4} ns floor, so this is a measurement rather than a bound.", floor);
    }
    println!();

    // Per payload, the bound rather than a value: see above.
    let d = floor.max(worst);
    println!("# WHAT THAT BOUNDS PER PAYLOAD, using the forward-crossing counts from the");
    println!("# counting build (a property of the descriptor, not of this machine):");
    println!();
    println!("{:<10} {:<9} {:>10} {:>18}", "payload", "direction", "forward", "guard bound ns");
    // From ffi/logs/rust/stage3-M2.log and the counts binary, warm contexts.
    for (pid, dir, n) in [
        ("P1.1", "encode", 2u64), ("P1.1", "decode", 1),
        ("P1.2", "encode", 8), ("P1.2", "decode", 1),
        ("P1.3", "encode", 3), ("P1.3", "decode", 1),
        ("P2.1", "encode", 7), ("P2.1", "decode", 1),
        ("P2.2", "encode", 2511), ("P2.2", "decode", 1),
        ("P3.1", "encode", 3), ("P3.1", "decode", 1),
        ("P6.1", "encode", 1401), ("P6.1", "decode", 1),
    ] {
        println!("{:<10} {:<9} {:>10} {:>17.1}", pid, dir, n, d * n as f64);
    }
    println!();
    println!("# The guard is per ENTRY POINT and the entry points are per message or per");
    println!("# chunk, never per field -- which is why the decode column is one crossing on");
    println!("# every payload however large it is. The encode column is where the count");
    println!("# grows, and it grows with the number of ELEMENT RUNS, not with the fields.");
    println!();
    println!("# AND AS A FRACTION OF THE WORK, which is the form that travels. The heaviest");
    println!("# row above is P2.2 encode: {:.1} ns against an encode this slice measures at", d * 2511.0);
    println!("# about 1.4 ms, so {:.3} percent. The decode column is one crossing on every",
             d * 2511.0 / 1_400_000.0 * 100.0);
    println!("# payload, so the guard cannot reach a tenth of a percent of any decode here.");
    println!("#");
    println!("# A host whose forward crossing is dearer does NOT pay more for the guard: the");
    println!("# guard is work inside the core, on the core's side of the boundary, and it is");
    println!("# the same load and branch whoever called. What changes per host is how many");
    println!("# entry points get called, and that is the crossing count above.");
}

#[inline(never)]
fn run_bare(n: usize) -> u64 {
    let mut acc = 0u64;
    for i in 0..n as u64 {
        acc = acc.wrapping_add(unsafe { ak_noop(i) });
    }
    acc
}

#[inline(never)]
fn run_bare2(n: usize) -> u64 {
    let mut acc = 0u64;
    for i in 0..n as u64 {
        acc = acc.wrapping_add(unsafe { ak_noop2(i) });
    }
    acc
}

#[inline(never)]
fn run_guarded(n: usize) -> u64 {
    let mut acc = 0u64;
    for i in 0..n as u64 {
        acc = acc.wrapping_add(unsafe { ak_noop_guarded(i) });
    }
    acc
}

fn black<T>(v: T) -> T {
    std::hint::black_box(v)
}

fn stats(v: &[f64]) -> (f64, f64, f64) {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (s[s.len() / 2], s[0], s[s.len() - 1])
}
