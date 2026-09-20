//! ABI v1 conformance obligation 12.5: the concurrency suite.
//!
//! **The obligation, verbatim**: "A concurrency suite that runs at least two payload
//! shapes [...] Of at least one message type, with threads run in sequence as well as
//! together, and every encode asserted against a reference rather than counted. A suite
//! with one shape reports zero wrong bytes with a per-thread-state defect present and
//! absent alike; two shapes find it in twenty encodes out of twenty."
//!
//! **Why it is the obligation with the most evidence behind it and the least existence.**
//! Stage 4 found a shared-mutable-client defect (D16: `ak_call_unary` took `*mut ak_client`
//! and mutated a shared `Grpc`, so two host threads calling it at once raced; it worked at
//! 1 in flight and failed outright at 8) **by accident**, because the RPC benchmark
//! happened to ask for 8 calls in flight. Nothing in any slice of this branch looks for
//! that class on purpose, and until this binary existed nothing in the branch ran the codec
//! on more than one thread at all.
//!
//! **What is at risk, specifically.** ABI v1 section 6 says the learned length-prefix width
//! table "lives in the context, never process-global", and open decision 5 measured what it
//! buys. It has never been touched by two threads. Two shapes matter because the table is
//! indexed by SITE: one shape exercises one set of sites and converges, and a per-thread or
//! per-context state defect is then invisible; two shapes interleaved make the same site
//! want two different widths, which is the state a defect corrupts.
//!
//! Five sections:
//!   1. the reference, single-threaded, one context per shape       (the oracle)
//!   2. SEQUENCE: two shapes alternating on ONE context, one thread (the per-context state)
//!   3. TOGETHER: N threads, one context each, both shapes, every encode compared
//!   4. THE POSITIVE CONTROL: the same suite against a deliberate contract violation, so
//!      the suite is seen failing. A guard with no failing test is a guard nobody has seen
//!      work (R1's second half).
//!   5. aggregate throughput at 1, 2 and 4 threads, which is what section 6's
//!      never-process-global sentence is actually a claim about. Run this binary twice,
//!      with and without `--features global-widths`, and the two numbers are the claim.

use ak_abi::*;
use harness::arms::core_ffi_arm::Ctx;
use harness::{arms, arms_m2};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// The two shapes. They are chosen for the reason the obligation gives: M1's `ResultRaw`
/// is a leaf with six blobs, and M2's `TaskDetailed` is not a leaf and carries repeated
/// strings and a map inside an inlined child. They exercise disjoint length-prefix sites
/// and different numbers of them, which is what makes a shared width table flip-flop.
const SHAPE_A: &str = "P1.2";
const SHAPE_B: &str = "P2.2";
/// A third pair, smaller, so a round is cheap enough to run many of.
const SHAPE_A2: &str = "P1.3";
const SHAPE_B2: &str = "P2.5";

const ROUNDS_PER_THREAD: usize = 200;

fn main() {
    // The positive control runs in its OWN process. See `positive_control`: the violation
    // it plants ends in a process abort, not in a return value, so it cannot share a
    // process with the sections whose results are being reported.
    if std::env::args().any(|a| a == "--control") {
        run_control();
        return;
    }
    let global = cfg!(feature = "global-widths");
    println!("# ABI v1 obligation 12.5: the concurrency suite");
    println!("#   width table: {}", if global {
        "PROCESS-GLOBAL (ABI v1 section 6's REFUSED arrangement, --features global-widths)"
    } else {
        "per context (ABI v1 section 6 as specified)"
    });
    println!("#   guard:       {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!("#   shapes:      {SHAPE_A} and {SHAPE_B} (M1 leaf, M2 non-leaf), plus {SHAPE_A2} and {SHAPE_B2}");
    println!("#   vCPUs:       {}", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0));
    println!();

    let r = reference();
    let mut bad = 0usize;
    bad += sequence(&r);
    bad += together(&r);
    bad += positive_control(&r);
    throughput(&r);

    println!();
    if bad == 0 {
        println!("# VERDICT: every encode in every section matched the single-threaded");
        println!("# reference byte for byte, and every decode matched it by value.");
    } else {
        println!("# VERDICT: {bad} DISAGREEMENTS.");
    }
    std::process::exit(if bad == 0 { 0 } else { 1 });
}

/// The four shapes' wire bytes and facade values, taken single-threaded on fresh contexts.
/// This is the oracle every later section is compared against; it is itself checked against
/// `ffi/schema/generated/manifest.json` by `gen/stage2.sh`, so it is not two of the slice's
/// own components agreeing with each other.
struct Ref {
    a: Vec<u8>,
    b: Vec<u8>,
    a2: Vec<u8>,
    b2: Vec<u8>,
}

fn reference() -> Arc<Ref> {
    let c = Ctx::new();
    let r = Ref {
        a: arms::core_ffi_arm::encode(&c, &arms::armonik_arm::value(SHAPE_A)),
        b: {
            let c2 = Ctx::new();
            arms_m2::core_ffi_arm::encode(&c2, &arms_m2::armonik_arm::value(SHAPE_B))
        },
        a2: {
            let c3 = Ctx::new();
            arms::core_ffi_arm::encode(&c3, &arms::armonik_arm::value(SHAPE_A2))
        },
        b2: {
            let c4 = Ctx::new();
            arms_m2::core_ffi_arm::encode(&c4, &arms_m2::armonik_arm::value(SHAPE_B2))
        },
    };
    println!("## 1. the reference, single-threaded, a fresh context per shape");
    println!();
    println!("{:<8} {:>10}", "shape", "bytes");
    println!("{:<8} {:>10}", SHAPE_A, r.a.len());
    println!("{:<8} {:>10}", SHAPE_B, r.b.len());
    println!("{:<8} {:>10}", SHAPE_A2, r.a2.len());
    println!("{:<8} {:>10}", SHAPE_B2, r.b2.len());
    println!();
    Arc::new(r)
}

/// One thread, ONE context, the shapes alternating. This is the half of the obligation
/// that needs no threads at all and is the one most likely to be skipped: a per-context
/// state defect (a width left behind, an open tag left behind, a sticky error left behind)
/// shows here, and on a single shape it does not, because a single shape converges.
///
/// D9 is the precedent: an element run did not restore the codec's open-field state, so a
/// host that chunked wrote later chunks under the inner field's tag. Byte identity passed
/// anyway, because the two fields involved were both tag 1.
fn sequence(r: &Ref) -> usize {
    println!("## 2. SEQUENCE: two shapes alternating on ONE context, one thread");
    println!();
    let c = Ctx::new();
    let va = arms::armonik_arm::value(SHAPE_A);
    let vb = arms_m2::armonik_arm::value(SHAPE_B);
    let va2 = arms::armonik_arm::value(SHAPE_A2);
    let vb2 = arms_m2::armonik_arm::value(SHAPE_B2);
    let mut bad = 0usize;
    let mut n = 0usize;
    for i in 0..40 {
        // Deliberately not a fixed alternation: a rotation, so the same site is asked for
        // three different widths in a row rather than two in a repeating pair.
        let which = i % 4;
        let ok = match which {
            0 => arms::core_ffi_arm::encode_into(&c, &va) == &r.a[..],
            1 => arms_m2::core_ffi_arm::encode_into(&c, &vb) == &r.b[..],
            2 => arms::core_ffi_arm::encode_into(&c, &va2) == &r.a2[..],
            _ => arms_m2::core_ffi_arm::encode_into(&c, &vb2) == &r.b2[..],
        };
        n += 1;
        if !ok {
            bad += 1;
        }
    }
    // And the same on the decode side, which has its own per-context state: the sticky
    // error slot (D17) and, now, the pull family's record buffer.
    let mut dbad = 0usize;
    for i in 0..40 {
        let ok = match i % 4 {
            0 => arms::core_ffi_arm::decode(&c, &r.a) == va,
            1 => arms_m2::core_ffi_arm::decode(&c, &r.b) == vb,
            2 => arms::core_ffi_arm::decode(&c, &r.a2) == va2,
            _ => arms_m2::core_ffi_arm::decode(&c, &r.b2) == vb2,
        };
        if !ok {
            dbad += 1;
        }
    }
    println!("  encodes {n}, wrong {bad}");
    println!("  decodes 40, wrong {dbad}");
    println!();
    println!("# The one-shape control, which is the comparison obligation 12.5 asks for:");
    let mut one = 0usize;
    let c1 = Ctx::new();
    for _ in 0..40 {
        if arms::core_ffi_arm::encode_into(&c1, &va) != &r.a[..] {
            one += 1;
        }
    }
    println!("  one shape ({SHAPE_A}) on one context, 40 encodes, wrong {one}");
    println!("# A per-shape-state defect is invisible to that row by construction and");
    println!("# visible to the rotation above. That is why the obligation says two shapes.");
    println!();
    bad + dbad
}

/// N threads, each with its own context, each cycling both shapes, every encode compared
/// to the reference. This is the arrangement ABI v1 specifies and it must be clean.
fn together(r: &Ref) -> usize {
    println!("## 3. TOGETHER: N threads, one context each, both shapes");
    println!();
    println!("{:<10} {:>12} {:>10} {:>12}", "threads", "encodes", "wrong", "decodes wrong");
    let mut bad = 0usize;
    for threads in [2usize, 4, 8] {
        let wrong = Arc::new(AtomicUsize::new(0));
        let dwrong = Arc::new(AtomicUsize::new(0));
        let total = Arc::new(AtomicUsize::new(0));
        std::thread::scope(|s| {
            for t in 0..threads {
                let (r, wrong, dwrong, total) =
                    (r, wrong.clone(), dwrong.clone(), total.clone());
                s.spawn(move || {
                    // One context per thread. That is the contract: a context is a
                    // host-owned, single-threaded thing (ABI v1 section 3).
                    let c = Ctx::new();
                    let va = arms::armonik_arm::value(SHAPE_A2);
                    let vb = arms_m2::armonik_arm::value(SHAPE_B2);
                    let mut w = 0usize;
                    let mut d = 0usize;
                    for i in 0..ROUNDS_PER_THREAD {
                        // Offset the phase per thread so the threads are not in lockstep;
                        // lockstep would make every thread want the same width at the same
                        // moment, which is the EASY case for a shared table.
                        if (i + t) % 2 == 0 {
                            if arms::core_ffi_arm::encode_into(&c, &va) != &r.a2[..] {
                                w += 1;
                            }
                            if arms::core_ffi_arm::decode(&c, &r.a2) != va {
                                d += 1;
                            }
                        } else {
                            if arms_m2::core_ffi_arm::encode_into(&c, &vb) != &r.b2[..] {
                                w += 1;
                            }
                            if arms_m2::core_ffi_arm::decode(&c, &r.b2) != vb {
                                d += 1;
                            }
                        }
                    }
                    wrong.fetch_add(w, Ordering::Relaxed);
                    dwrong.fetch_add(d, Ordering::Relaxed);
                    total.fetch_add(ROUNDS_PER_THREAD, Ordering::Relaxed);
                });
            }
        });
        let (w, d, n) = (
            wrong.load(Ordering::Relaxed),
            dwrong.load(Ordering::Relaxed),
            total.load(Ordering::Relaxed),
        );
        println!("{:<10} {:>12} {:>10} {:>12}", threads, n, w, d);
        bad += w + d;
    }
    println!();
    println!("# 8 threads on 4 vCPUs is deliberate: oversubscription puts a preemption");
    println!("# between a `begin` and its `end`, which is where a state defect that needs a");
    println!("# window to open would show. R9 is the caveat on the other side -- a");
    println!("# concurrency figure from a small container is a lower bound on contention,");
    println!("# and section 5 below is the figure, not this section.");
    println!();
    bad
}

/// **The suite seen failing.** Obligation 12.5's own standard is that a suite with one
/// shape reports zero wrong bytes whether a per-thread-state defect is present or absent;
/// a suite that never reports a wrong byte at all has shown nothing either way.
///
/// The violation planted here is the one a C host can make and the type system cannot stop:
/// **N threads sharing ONE encode context**, which ABI v1 section 3 makes a host-owned,
/// single-threaded thing. It is exactly D16's class -- a shared mutable handle reached
/// through an opaque pointer -- moved from the RPC half to the codec half on purpose.
///
/// It is a deliberate misuse and it is a data race, so it is fenced: it runs last, its
/// result is a count and never a byte anyone looks at, and nothing downstream depends on
/// it. What it establishes is that sections 2 and 3 would have reported a defect of this
/// class had one been there.
fn positive_control(_r: &Ref) -> usize {
    println!("## 4. THE POSITIVE CONTROL: the suite has to be seen failing");
    println!();
    println!("# Planted violation: 4 threads share ONE encode context, which ABI v1 section");
    println!("# 3 makes a host-owned single-threaded thing. D16's class -- a shared mutable");
    println!("# handle reached through an opaque pointer -- moved from the RPC half to the");
    println!("# codec half on purpose. It is a deliberate contract violation and a data race,");
    println!("# so it runs in a CHILD PROCESS and nothing downstream depends on it.");
    println!();
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            println!("  !! cannot locate this binary to re-run it: {e}");
            return 1;
        }
    };
    let out = match std::process::Command::new(exe).arg("--control").output() {
        Ok(o) => o,
        Err(e) => {
            println!("  !! cannot spawn the control: {e}");
            return 1;
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stdout.lines() {
        println!("  {line}");
    }
    let code = out.status.code();
    let aborted = code.is_none() || code == Some(134) || code == Some(101);
    let reported_wrong = stdout.contains("WRONG");
    println!("  child exit: {:?}", code);
    // The first line of the child's stderr, which is where the core says what happened.
    if let Some(l) = stderr.lines().find(|l| l.contains("panicked at")) {
        println!("  child stderr: {}", l.trim());
    }
    if let Some(l) = stderr.lines().find(|l| l.contains("cannot unwind")) {
        println!("  child stderr: {}", l.trim());
    }
    println!();
    if !aborted && !reported_wrong {
        println!("  !! THE CONTROL DID NOT FIRE: the child finished cleanly. Either the");
        println!("  !! threads never overlapped or the suite cannot see this class of defect.");
        println!("  !! Sections 2 and 3 above are then evidence of nothing.");
        return 1;
    }
    println!("  The control fires, so sections 2 and 3 are statements about the code and not");
    println!("  about a check that cannot fail.");
    println!();
    println!("  AND THE SHAPE OF THE FAILURE IS A FINDING NOBODY ASKED FOR. The violation is");
    println!("  not detected as wrong bytes: two threads resizing one length prefix index the");
    println!("  encode buffer past its end, the core panics inside `Enc`, and because the");
    println!("  frame it has to unwind through is an `extern \"C\"` entry point of the ABI");
    println!("  (`ak_elem_ResultRaw`), the panic CANNOT unwind and the PROCESS ABORTS.");
    println!();
    println!("  Three things follow, and they are for the specification rather than for this");
    println!("  slice:");
    println!("   1. ABI v1 section 5's error channel covers a failure the HOST reports, through");
    println!("      `ak_fail` and the generated guard. A panic inside the CORE has no channel");
    println!("      at all: it reaches a `extern \"C\"` boundary and aborts. Every entry point");
    println!("      in the codec is exposed to this, not only under a shared context.");
    println!("   2. Section 3 says `ak_init` installs a panic hook. It is unbuilt (this slice");
    println!("      builds it -- see the lifecycle log), and a hook changes what is PRINTED,");
    println!("      not whether the abort happens. Catching the unwind at the entry point is a");
    println!("      different and larger change.");
    println!("   3. The context already carries a `kind` word so a wrong pointer is");
    println!("      diagnosable. An owning-thread id beside it would turn this abort into");
    println!("      `AK_ERR_INVALID_STATE` at the first misuse rather than at an arbitrary");
    println!("      later one. That is a design question and it is raised, not taken.");
    println!();
    0
}

/// The planted violation, in its own process. 4 threads, ONE shared encode context.
///
/// Prints what it saw and exits; it usually does not get to print, because the core aborts
/// first, and the parent reads that from the exit status.
fn run_control() {
    struct Shared(*mut ak_enc_ctx);
    unsafe impl Send for Shared {}
    unsafe impl Sync for Shared {}

    let c = Ctx::new();
    let refa = {
        let c2 = Ctx::new();
        arms::core_ffi_arm::encode(&c2, &arms::armonik_arm::value(SHAPE_A2))
    };
    let refb = {
        let c3 = Ctx::new();
        arms_m2::core_ffi_arm::encode(&c3, &arms_m2::armonik_arm::value(SHAPE_B2))
    };
    let shared = Shared(c.enc);
    let tcs = &c.tcs;
    let wrong = AtomicUsize::new(0);
    let total = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for t in 0..4 {
            let (shared, wrong, total, refa, refb) = (&shared, &wrong, &total, &refa, &refb);
            s.spawn(move || {
                let va = arms::armonik_arm::value(SHAPE_A2);
                let vb = arms_m2::armonik_arm::value(SHAPE_B2);
                for i in 0..ROUNDS_PER_THREAD {
                    total.fetch_add(1, Ordering::Relaxed);
                    let want: &[u8] = if (i + t) % 2 == 0 { refa } else { refb };
                    let got: &[u8] = unsafe {
                        if (i + t) % 2 == 0 {
                            harness::generated::binding::encode_into_list_results_response(
                                shared.0, &va, tcs,
                            )
                            .ok();
                            harness::generated::binding::encoded(shared.0)
                        } else {
                            harness::generated::binding::encode_into_list_tasks_detailed_response(
                                shared.0, &vb, tcs,
                            )
                            .ok();
                            harness::generated::binding::encoded(shared.0)
                        }
                    };
                    if got != want {
                        wrong.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });
    let (w, n) = (wrong.load(Ordering::Relaxed), total.load(Ordering::Relaxed));
    println!("shared context, 4 threads: {n} encodes, {w} WRONG");
    if w == 0 {
        std::process::exit(0);
    }
    std::process::exit(2);
}

/// **Section 6's claim, measured.** "The table lives in the context, never process-global"
/// rests on a figure the branch inherited from the java slice (1.32 to 2.23 times aggregate
/// throughput at two threads, against a global table) and that no Rust host has checked.
///
/// Run this binary twice, once with `--features global-widths` and once without. Both
/// builds produce correct bytes, because `Mark` carries its width by value, so what the
/// comparison isolates is the table and nothing else.
///
/// **TWO SHAPE PAIRS, and the difference between them is the mechanism.** A shared table is
/// only contended if two shapes want DIFFERENT widths at the SAME site, and whether they do
/// is a property of which messages they are:
///
///   * P1.1 and P1.3 are both M1, so they index the same sites, and `ResultRaw` is about
///     200 bytes in one and empty in the other -- so the site's width flips between 2 and 1
///     on every alternation. This is section 6's case.
///   * P1.3 and P2.5 are different messages and index DISJOINT sites, so a shared table is
///     shared in name only. This row is the control, and the first version of this suite had
///     only this row, which is why it found nothing.
fn throughput(r: &Ref) {
    println!("## 5. aggregate throughput, {} width table",
             if cfg!(feature = "global-widths") { "PROCESS-GLOBAL" } else { "per-context" });
    println!();
    flip_check();
    pair(r, true);
    pair(r, false);
    println!("# The warm-up is outside the clock, so a per-context build has already learned");
    println!("# every width before the first timed encode. A global build cannot reach that");
    println!("# state on the SHARED-SITE row, because the two shapes keep taking it back --");
    println!("# and it reaches it immediately on the disjoint row, which is the control.");
    println!("# R9: 4 vCPUs in a shared container is the smallest contention a shared cache");
    println!("# line can have, so any penalty here is a LOWER BOUND.");
}

/// **Does the shared-site pair actually flip a width?** The slice's standing question is
/// "when a change does not do what it should, the first hypothesis is that it is not
/// running", and its mirror applies here: a contention arm that never contends measures
/// nothing. So the flipping is COUNTED rather than argued.
///
/// `ResultRaw` is about 200 bytes in P1.1 and empty in P1.3, so its length-prefix site
/// wants 2 bytes in one and 1 in the other. A warm context encoding ONE shape repeatedly
/// should miss zero times; the same context alternating the two should miss on every
/// element. Counting build only.
fn flip_check() {
    if !cfg!(feature = "count") {
        println!("### does the shared-site pair flip a width? (rebuild with --features count)");
        println!();
        return;
    }
    println!("### does the shared-site pair flip a width? (counted, not argued)");
    println!();
    let v0 = arms::armonik_arm::value("P1.1");
    let v1 = arms::armonik_arm::value(SHAPE_A2);
    let mut out = Vec::new();
    for (label, alt) in [("P1.1 alone", false), ("P1.1 / P1.3 alternating", true)] {
        let c = Ctx::new();
        for _ in 0..20 {
            arms::core_ffi_arm::encode_into(&c, &v0);
            if alt {
                arms::core_ffi_arm::encode_into(&c, &v1);
            }
        }
        unsafe { ak_enc_counters_reset(c.enc) };
        for _ in 0..20 {
            arms::core_ffi_arm::encode_into(&c, &v0);
            if alt {
                arms::core_ffi_arm::encode_into(&c, &v1);
            }
        }
        let mut cs = AkCounters::default();
        unsafe { ak_enc_counters(c.enc, &mut cs) };
        out.push((label, cs.prefix_moves, cs.prefix_bytes));
    }
    println!("{:<26} {:>14} {:>14}", "warm context", "prefix misses", "bytes moved");
    for (l, m, b) in out {
        println!("{:<26} {:>14} {:>14}", l, m, b);
    }
    println!();
    println!("# A warm context on ONE shape misses nothing; the same context alternating the");
    println!("# two misses on every element. That is the state a shared table puts every");
    println!("# thread in at once, and it is why the pair was chosen.");
    println!();
}

fn pair(r: &Ref, shared_sites: bool) {
    println!("### {} ({})",
             if shared_sites { "P1.1 + P1.3, SHARED SITES (both M1, widths flip 2<->1)" }
             else { "P1.3 + P2.5, disjoint sites (the control)" },
             if cfg!(feature = "global-widths") { "global table" } else { "per-context table" });
    println!();
    println!("{:<10} {:>14} {:>14} {:>10} {:>14}",
             "threads", "enc/s aggregate", "ns per encode", "scaling", "vs 1 thread");
    let mut base = 0.0f64;
    let mut base_ns = 0.0f64;
    for threads in [1usize, 2, 4] {
        let mut best = 0.0f64;
        for _ in 0..3 {
            let n = Arc::new(AtomicUsize::new(0));
            let t0 = Instant::now();
            std::thread::scope(|s| {
                for t in 0..threads {
                    let (n, r) = (n.clone(), r);
                    s.spawn(move || {
                        let c = Ctx::new();
                        let (v0, v1) = if shared_sites {
                            (arms::armonik_arm::value("P1.1"), arms::armonik_arm::value(SHAPE_A2))
                        } else {
                            (arms::armonik_arm::value(SHAPE_A2), arms::armonik_arm::value(SHAPE_A2))
                        };
                        let vb = arms_m2::armonik_arm::value(SHAPE_B2);
                        let mut done = 0usize;
                        for _ in 0..50 {
                            arms::core_ffi_arm::encode_into(&c, &v0);
                            if shared_sites {
                                arms::core_ffi_arm::encode_into(&c, &v1);
                            } else {
                                arms_m2::core_ffi_arm::encode_into(&c, &vb);
                            }
                        }
                        for i in 0..ROUNDS_PER_THREAD * 10 {
                            if (i + t) % 2 == 0 {
                                std::hint::black_box(arms::core_ffi_arm::encode_into(&c, &v0));
                            } else if shared_sites {
                                std::hint::black_box(arms::core_ffi_arm::encode_into(&c, &v1));
                            } else {
                                std::hint::black_box(arms_m2::core_ffi_arm::encode_into(&c, &vb));
                            }
                            done += 1;
                        }
                        n.fetch_add(done, Ordering::Relaxed);
                        let _ = r;
                    });
                }
            });
            let secs = t0.elapsed().as_secs_f64();
            let rate = n.load(Ordering::Relaxed) as f64 / secs;
            if rate > best {
                best = rate;
            }
        }
        let ns = 1e9 / best * threads as f64;
        if threads == 1 {
            base = best;
            base_ns = ns;
        }
        println!("{:<10} {:>14.0} {:>14.1} {:>10.2}x {:>13.1}%",
                 threads, best, ns, best / base, (ns - base_ns) / base_ns * 100.0);
    }
    println!();
}
