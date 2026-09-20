//! ABI v1 section 3: the lifecycle, built and exercised.
//!
//! **Why this is here.** Section 3 is the part of the specification every slice skipped,
//! and for a reason that is itself the finding: the codec half needs none of it, so nobody
//! built it, so section 3's central claim -- "**every other entry point requires `ak_init`
//! to have returned successfully, the codec included**" -- had never been exercised
//! anywhere in this branch. `findings/rust.md` records it as "specified and not built" and
//! every other slice's list says the same.
//!
//! **What is real here and what is not**, stated first because the alternative is a reader
//! discovering it:
//!
//! | section 3 says | built? |
//! |---|---|
//! | `ak_init` with options, an `ak_err` out-parameter, the flags | yes |
//! | the ABI-version check, made by the side that knows what it was generated against | yes |
//! | idempotent under identical options, `AK_ALREADY_INITIALIZED` as a SUCCESS | yes |
//! | a second call with different options fails | yes |
//! | no `ak_shutdown` | yes, by not existing |
//! | the build id, because two copies in one process split-brain Rust's globals | yes |
//! | the log bridge, and `AK_INIT_OWN_LOGGING` to decline it | yes |
//! | the panic hook, and `AK_INIT_NO_PANIC_HOOK` to decline it | yes |
//! | every other entry point returns `AK_ERR_UNINITIALIZED` | yes, behind `init-guard`, and PRICED |
//! | the rustls crypto provider installed by name | **no**: this build does not link rustls |
//! | `tracing::set_global_default` and `log::set_logger` | **no**: the bridge is the ABI's `ak_log_fn`, not those crates |
//! | configuration precedence: setter > environment > JSON > defaults | **no**: it belongs to `ak_context_new`, which is the RPC half |
//! | `worker_threads` from `ak_runtime_opts` with a small explicit default | **no**: same |
//!
//! The four "no"s are not oversights and they are not cheap to fake. A one-line install
//! nobody has run is not evidence.
//!
//! **Several cases need a FRESH PROCESS**, because `ak_init` is one-shot per process by
//! construction and there is no `ak_shutdown` to undo it. So this binary re-runs itself
//! with a case name, and the parent reports each child's verdict. That is not a workaround;
//! it is what "one-shot per process" means, and a suite that tested it any other way would
//! be testing something else.

use ak_abi::*;
use std::ffi::CStr;
use std::sync::atomic::{AtomicUsize, Ordering};

static LOG_LINES: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn host_log(ctx: *mut std::ffi::c_void, _level: u32, msg: *const u8, len: usize) {
    use std::io::Write;
    LOG_LINES.fetch_add(1, Ordering::Relaxed);
    let s = String::from_utf8_lossy(std::slice::from_raw_parts(msg, len));
    println!("    [host log, ctx={:?}] {}", ctx, s);
    // Flushed, because the caller may be on its way to an abort and buffered stdout would
    // be lost with it. That is the panic case, and it is the case the sink exists for.
    let _ = std::io::stdout().flush();
}

fn opts(flags: u32, log: Option<ak_log_fn>) -> ak_init_opts {
    ak_init_opts {
        abi_version: AK_ABI_VERSION,
        flags,
        log,
        log_ctx: std::ptr::null_mut(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(case) = args.iter().skip(1).find(|a| !a.starts_with("--")) {
        std::process::exit(child(case));
    }

    println!("# ABI v1 section 3: the lifecycle");
    println!("#   init-guard: {}", if cfg!(feature = "init-guard") {
        "ON -- every entry point checks (section 3 as specified)"
    } else {
        "off -- entry points do not check; the guard's PRICE is section 4"
    });
    println!("#   build id:   {}", unsafe {
        CStr::from_ptr(ak_build_id()).to_string_lossy()
    });
    println!("#   ak_abi_version(): {}, the host was generated against {}",
             unsafe { ak_abi_version() }, AK_ABI_VERSION);
    println!();
    println!("# Each case below is a FRESH PROCESS. `ak_init` is one-shot and there is no");
    println!("# `ak_shutdown`, so a suite that ran two cases in one process would be");
    println!("# measuring the second case against the first case's installs.");
    println!();
    println!("{:<28} {:<6} {}", "case", "exit", "what it establishes");

    let cases: &[(&str, &str)] = &[
        ("uninit-first", "an entry point BEFORE ak_init"),
        ("init-ok", "ak_init returns AK_OK and ak_initialized() flips"),
        ("init-twice-same", "the same options again: AK_ALREADY_INITIALIZED, a SUCCESS"),
        ("init-twice-differ", "different options: refused, AK_DETAIL_OPTS_DIFFER"),
        ("init-bad-version", "a host generated against another ABI: AK_ERR_ABI"),
        ("init-null", "a null options pointer: AK_ERR_INVALID_STATE, not a crash"),
        ("log-bridge", "the host's ak_log_fn receives a line"),
        ("log-declined", "AK_INIT_OWN_LOGGING: no line reaches the host"),
        ("panic-hook", "a panic IN THE CORE reaches the host's log, then aborts"),
        ("no-panic-hook", "AK_INIT_NO_PANIC_HOOK: no line, and still an abort"),
        ("host-panic-own-hook", "a HOST panic is not the core's; two copies of std"),
        ("codec-after-init", "the codec works normally once initialised"),
        ("two-components", "two components, two flag sets: the first caller wins"),
        ("init-races", "8 threads calling ak_init at once: exactly one installs"),
    ];
    let exe = std::env::current_exe().expect("current_exe");
    let mut bad = 0;
    for (name, what) in cases {
        let out = std::process::Command::new(&exe).arg(name).output().expect("spawn");
        let code = out.status.code();
        let stdout = String::from_utf8_lossy(&out.stdout);
        // The two panic cases cannot exit 0: the panic is on the far side of an
        // `extern "C"` frame and the process aborts. Their verdict is "the child aborted
        // AND the host's sink did (or did not) get the line first".
        let ok = match *name {
            "panic-hook" => code.is_none() && stdout.contains("[host log"),
            "no-panic-hook" => code.is_none() && !stdout.contains("[host log"),
            _ => code == Some(0),
        };
        if !ok {
            bad += 1;
        }
        println!("{:<28} {:<6} {}", name, if ok { "ok" } else { "FAIL" }, what);
        for l in stdout.lines() {
            println!("    {l}");
        }
        if code.is_none() {
            let se = String::from_utf8_lossy(&out.stderr);
            if let Some(l) = se.lines().find(|l| l.contains("cannot unwind")) {
                println!("    (child aborted) {}", l.trim());
            } else {
                println!("    (child aborted, signal)");
            }
        }
        if !ok {
            for l in String::from_utf8_lossy(&out.stderr).lines().take(3) {
                println!("    ! {l}");
            }
        }
    }

    println!();
    if bad == 0 {
        println!("# VERDICT: section 3's lifecycle behaves as specified on every case built.");
    } else {
        println!("# VERDICT: {bad} case(s) FAILED.");
    }
    std::process::exit(if bad == 0 { 0 } else { 1 });
}

/// One case, in its own process. Returns the exit code.
fn child(case: &str) -> i32 {
    let ok = |b: bool| if b { 0 } else { 1 };
    unsafe {
        match case {
            // ---- the claim the whole section rests on -------------------------------
            "uninit-first" => {
                // A codec entry point called before `ak_init`. With `init-guard` on this
                // must be AK_ERR_UNINITIALIZED; with it off it succeeds, and the suite
                // says which build it is rather than pretending the guard is always there.
                assert_eq!(ak_initialized(), 0, "nothing has initialised yet");
                let c = ak_enc_ctx_new();
                let v = facade::build::payload_p1_1();
                let tcs = harness::generated::binding::Tcs::trusted();
                let r = harness::generated::binding::encode_into_list_results_response(c, &v, &tcs);
                ak_enc_ctx_free(c);
                if cfg!(feature = "init-guard") {
                    println!("guard ON: encode before ak_init -> {:?} (want Err({}))",
                             r.as_ref().map(|_| ()).map_err(|e| *e), AK_ERR_UNINITIALIZED);
                    ok(matches!(r, Err(e) if e == AK_ERR_UNINITIALIZED))
                } else {
                    println!("guard off: encode before ak_init SUCCEEDED, which is what this");
                    println!("build means. Section 3's rule is unenforced unless init-guard is on.");
                    ok(r.is_ok())
                }
            }

            // ---- the state machine --------------------------------------------------
            "init-ok" => {
                let mut e = ak_err::default();
                let o = opts(AK_INIT_NO_CRYPTO, None);
                let rc = ak_init(&o, &mut e);
                println!("ak_init -> {rc} (AK_OK={AK_OK}), detail {}", e.detail);
                println!("ak_initialized() -> {}", ak_initialized());
                ok(rc == AK_OK && e.code == AK_OK && ak_initialized() == 1)
            }
            "init-twice-same" => {
                let o = opts(AK_INIT_NO_CRYPTO, None);
                let mut e = ak_err::default();
                let a = ak_init(&o, &mut e);
                let b = ak_init(&o, &mut e);
                println!("first {a}, second {b} (AK_ALREADY_INITIALIZED={AK_ALREADY_INITIALIZED})");
                println!("# and it is a SUCCESS, not an error: a host library that initialises");
                println!("# defensively must not fail because another one got there first.");
                ok(a == AK_OK && b == AK_ALREADY_INITIALIZED && b > 0)
            }
            "init-twice-differ" => {
                let mut e = ak_err::default();
                let a = ak_init(&opts(AK_INIT_NO_CRYPTO, None), &mut e);
                let b = ak_init(&opts(AK_INIT_NO_CRYPTO | AK_INIT_OWN_LOGGING, None), &mut e);
                println!("first {a}, second {b}, detail {} (AK_DETAIL_OPTS_DIFFER={AK_DETAIL_OPTS_DIFFER})",
                         e.detail);
                println!("# The one-shot installs cannot be redone, which is also why there is");
                println!("# no ak_shutdown.");
                ok(a == AK_OK && b == AK_ERR_INVALID_STATE && e.detail == AK_DETAIL_OPTS_DIFFER)
            }
            "init-bad-version" => {
                let mut e = ak_err::default();
                let mut o = opts(AK_INIT_NO_CRYPTO, None);
                o.abi_version = AK_ABI_VERSION + 7;
                let rc = ak_init(&o, &mut e);
                println!("ak_init(abi_version={}) -> {rc}, detail {}", o.abi_version, e.detail);
                println!("ak_initialized() -> {} (must still be 0)", ak_initialized());
                println!("# The check is made by the side that KNOWS what it was generated");
                println!("# against, once, at the only point where failing is cheap.");
                ok(rc == AK_ERR_ABI && e.detail == AK_DETAIL_ABI_MISMATCH && ak_initialized() == 0)
            }
            "init-null" => {
                let mut e = ak_err::default();
                let rc = ak_init(std::ptr::null(), &mut e);
                println!("ak_init(NULL) -> {rc}, detail {}", e.detail);
                // And with a null err too, which a C host is entitled to pass.
                let rc2 = ak_init(std::ptr::null(), std::ptr::null_mut());
                println!("ak_init(NULL, NULL) -> {rc2}");
                ok(rc == AK_ERR_INVALID_STATE && e.detail == AK_DETAIL_NULL_ARG
                    && rc2 == AK_ERR_INVALID_STATE)
            }

            // ---- the installs -------------------------------------------------------
            "log-bridge" => {
                let mut e = ak_err::default();
                let rc = ak_init(&opts(AK_INIT_NO_CRYPTO, Some(host_log)), &mut e);
                let msg = b"a line from the core";
                let delivered = ak_log_test(2, msg.as_ptr(), msg.len());
                println!("ak_init -> {rc}; ak_log_test delivered={delivered}, lines={}",
                         LOG_LINES.load(Ordering::Relaxed));
                ok(rc == AK_OK && delivered == 1 && LOG_LINES.load(Ordering::Relaxed) == 1)
            }
            "log-declined" => {
                let mut e = ak_err::default();
                let rc = ak_init(&opts(AK_INIT_NO_CRYPTO | AK_INIT_OWN_LOGGING, Some(host_log)),
                                 &mut e);
                let msg = b"a line the host did not ask for";
                let delivered = ak_log_test(2, msg.as_ptr(), msg.len());
                println!("ak_init(OWN_LOGGING) -> {rc}; delivered={delivered}, lines={}",
                         LOG_LINES.load(Ordering::Relaxed));
                println!("# The host passed a sink AND the flag; the flag wins, because the");
                println!("# flag is the host saying it owns the process log.");
                ok(rc == AK_OK && delivered == 0 && LOG_LINES.load(Ordering::Relaxed) == 0)
            }
            // The hook covers a panic raised INSIDE the core. It has to be tested that
            // way, and the reason is the finding: a cdylib carries its own copy of `std`,
            // so the core's hook and the host's hook are two different globals. A Rust
            // host's own panics never reach the core's hook and were never meant to.
            //
            // This case does not return: the panic is on the far side of an `extern "C"`
            // frame, the unwind is refused there and the process aborts. So the parent
            // reads the verdict from the child's OUTPUT and its exit status, and the case
            // passes when the host's sink got the message BEFORE the abort -- which is the
            // whole of what the hook buys.
            "panic-hook" => {
                let mut e = ak_err::default();
                ak_init(&opts(AK_INIT_NO_CRYPTO, Some(host_log)), &mut e);
                println!("WANT: one line through the host sink, then an abort");
                ak_panic_test();
                println!("UNREACHABLE: ak_panic_test returned");
                1
            }
            "no-panic-hook" => {
                let mut e = ak_err::default();
                ak_init(&opts(AK_INIT_NO_CRYPTO | AK_INIT_NO_PANIC_HOOK, Some(host_log)), &mut e);
                println!("WANT: no line through the host sink, and still an abort");
                ak_panic_test();
                println!("UNREACHABLE: ak_panic_test returned");
                1
            }
            // A Rust host's own panic, for the contrast. The core's hook is not involved
            // and the host's own hook fires: two copies of `std`, two hook globals.
            "host-panic-own-hook" => {
                let seen = std::sync::Arc::new(AtomicUsize::new(0));
                let s2 = seen.clone();
                std::panic::set_hook(Box::new(move |_| {
                    s2.fetch_add(1, Ordering::Relaxed);
                }));
                let mut e = ak_err::default();
                ak_init(&opts(AK_INIT_NO_CRYPTO, Some(host_log)), &mut e);
                let r = std::panic::catch_unwind(|| panic!("deliberate, from host code"));
                let (mine, theirs) = (seen.load(Ordering::Relaxed), LOG_LINES.load(Ordering::Relaxed));
                println!("host hook fired {mine} time(s); the CORE's sink got {theirs} line(s)");
                println!("# The core installed a hook and it did not fire, because the cdylib");
                println!("# carries its own `std`. That is not a defect: the hook is for panics");
                println!("# in the CORE, which is where the host cannot see them.");
                ok(r.is_err() && mine == 1 && theirs == 0)
            }

            // ---- and that none of it broke the codec --------------------------------
            "codec-after-init" => {
                // NO explicit `ak_init` here: `Ctx::new()` does it, which is the shape a
                // host LIBRARY has. The first draft of this case called `ak_init` itself
                // with one flag set and then let `Ctx::new()` call it with another, and
                // the second call was refused -- which is the rule working, and which is
                // what the `two-components` case below now covers on purpose.
                let c = harness::arms::core_ffi_arm::Ctx::new();
                let v = harness::arms::armonik_arm::value(harness::arms::P1_2);
                let b = harness::arms::core_ffi_arm::encode(&c, &v);
                let back = harness::arms::core_ffi_arm::decode(&c, &b);
                let want = harness::arms::prost_arm::encode(&harness::arms::prost_arm::value(
                    harness::arms::P1_2,
                ));
                println!("initialised by Ctx::new(); P1.2 encode {} B, prost {} B, equal={}, round trip={}",
                         b.len(), want.len(), b == want, back == v);
                ok(ak_initialized() == 1 && b == want && back == v)
            }

            // ---- what the one-shot rule means for a process with two components ------
            //
            // Found by this suite rather than written for it: the first draft of
            // `codec-after-init` called `ak_init` with one flag set and then let
            // `Ctx::new()` call it with another, and the second call was refused. That is
            // section 3 working exactly as written, and it has a consequence the
            // specification does not spell out.
            "two-components" => {
                let mut e = ak_err::default();
                // Component A, say a logging shim, wants the core's log bridge.
                let a = ak_init(&opts(AK_INIT_NO_CRYPTO, Some(host_log)), &mut e);
                // Component B, say a codec wrapper in the same process, keeps its own
                // panic hook. Both are initialising defensively, neither is wrong, and
                // the flags differ.
                let b = ak_init(&opts(AK_INIT_NO_CRYPTO | AK_INIT_NO_PANIC_HOOK, Some(host_log)),
                                &mut e);
                println!("component A -> {a}; component B -> {b}, detail {}", e.detail);
                println!("# THE FLAGS ARE A PROCESS-WIDE NEGOTIATION AND THE FIRST CALLER WINS.");
                println!("# Section 3 says a second call with different options fails, and treats");
                println!("# that as obviously right because the one-shot installs cannot be");
                println!("# redone. It is right, and it also means two independent components in");
                println!("# one process cannot both choose -- the second gets a hard failure for");
                println!("# asking, not the first caller's settings. Two hosts loading the same");
                println!("# core is the normal case for a shared library. Raised, not taken: the");
                println!("# alternatives (ignore the later flags, or merge them) each give up");
                println!("# something section 3 is deliberately buying.");
                ok(a == AK_OK && b == AK_ERR_INVALID_STATE && e.detail == AK_DETAIL_OPTS_DIFFER)
            }

            // ---- the one-shot rule under contention ---------------------------------
            "init-races" => {
                // "It is idempotent under identical options" has to hold when eight threads
                // race for it, not only when one host calls it twice. Exactly one must
                // return AK_OK and the rest AK_ALREADY_INITIALIZED, and no caller may
                // return before the installs are visible.
                let oks = std::sync::Arc::new(AtomicUsize::new(0));
                let already = std::sync::Arc::new(AtomicUsize::new(0));
                let other = std::sync::Arc::new(AtomicUsize::new(0));
                let early = std::sync::Arc::new(AtomicUsize::new(0));
                std::thread::scope(|s| {
                    for _ in 0..8 {
                        let (oks, already, other, early) =
                            (oks.clone(), already.clone(), other.clone(), early.clone());
                        s.spawn(move || {
                            let mut e = ak_err::default();
                            let rc = ak_init(&opts(AK_INIT_NO_CRYPTO, None), &mut e);
                            // Every caller, not only the first, must see the installs done
                            // by the time it returns.
                            if ak_initialized() != 1 {
                                early.fetch_add(1, Ordering::Relaxed);
                            }
                            match rc {
                                AK_OK => oks.fetch_add(1, Ordering::Relaxed),
                                AK_ALREADY_INITIALIZED => already.fetch_add(1, Ordering::Relaxed),
                                _ => other.fetch_add(1, Ordering::Relaxed),
                            };
                        });
                    }
                });
                let (o, a, x, e2) = (
                    oks.load(Ordering::Relaxed),
                    already.load(Ordering::Relaxed),
                    other.load(Ordering::Relaxed),
                    early.load(Ordering::Relaxed),
                );
                println!("8 threads: {o} AK_OK, {a} AK_ALREADY_INITIALIZED, {x} other");
                println!("callers that returned before the installs were visible: {e2}");
                ok(o == 1 && a == 7 && x == 0 && e2 == 0)
            }

            other => {
                eprintln!("unknown case {other}");
                2
            }
        }
    }
}
