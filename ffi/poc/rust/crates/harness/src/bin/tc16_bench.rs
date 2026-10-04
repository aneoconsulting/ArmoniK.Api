//! D19 instrumentation (CONTAINER: not a campaign result): the UTF-16 transcoder alone, old
//! (`ak_tc_utf16_scalar`, the pre-D19 two-pass scalar code) against new (`ak_tc_utf16`,
//! simdutf), both called through the function pointer the core calls, in ONE process,
//! interleaved per round. A third column is the additive export `ak_utf16_to_utf8` on the
//! same input, and with `AK_D19_BASE_CORE` a fourth, the pre-D19 core binary's own
//! `ak_tc_utf16` (the control for the scalar export). Capacity ample (3 bytes per unit), so no grow: the transcoder only.
//!
//! Per cell (content set x length in code units): a pool of distinct strings (at least 16,
//! at most 4 MiB of UTF-16 or 1024 strings), walked in order; each measurement runs the
//! pool enough times for about `AK_D19_MS` ms (default 40), `AK_D19_ROUNDS` rounds
//! (default 7), the order of the three arms rotated per round. Reported: ns per string and
//! GB/s of UTF-16 input (2 bytes per unit), median and min..max over rounds, and the
//! median of the per-round new/old ratios.
use ak_abi::*;
use core::ffi::c_void;
use harness::d19::*;
use std::hint::black_box;
use std::time::Instant;

unsafe extern "C" fn no_grow(_s: *mut c_void, _w: i32, _d: *mut *mut u8, _c: *mut i32) -> i32 {
    AK_ERR_CAPACITY
}

fn med(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let ms: f64 = std::env::var("AK_D19_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(40.0);
    let rounds: usize = std::env::var("AK_D19_ROUNDS").ok().and_then(|s| s.parse().ok()).unwrap_or(7);
    let _init = harness::arms::core_ffi_arm::Ctx::new();
    let new_tc = unsafe { ak_tc_utf16() };
    let old_tc = unsafe { ak_tc_utf16_scalar() };
    // Arm 3, when AK_D19_BASE_CORE names the pre-D19 core: its own ak_tc_utf16, the control
    // that the scalar export is the pre-D19 code (same bytes: tc16_diff; same cost: here).
    let base = base_core_tc16();
    let base_tc = base.as_ref().map(|b| b.1).unwrap_or(old_tc);
    let narms = if base.is_some() { 4 } else { 3 };
    let sets = [Set::Ascii, Set::Latin1, Set::Wide, Set::Astral, Set::Mixed, Set::Lone];
    let lens = [8usize, 32, 128, 1024, 65536];
    println!("# D19 tc16_bench: container instrumentation; rounds {rounds}, ~{ms} ms per measurement; arms rotated per round");
    match &base {
        Some((p, f)) => println!("# pre-D19 core loaded RTLD_LOCAL: {p} (ak_tc_utf16 {:p}); arm pre-D19-core", *f as *const ()),
        None => println!("# pre-D19 core: not loaded (AK_D19_BASE_CORE unset)"),
    }
    println!("# set | units | strings in pool | out B/str | arm | ns/string median [min..max] | GB/s (UTF-16 in) median [min..max]");
    let mut r = Rng(0xBE_0C4);
    for &set in &sets {
        for &n in &lens {
            let k = (4 << 20) / (2 * n);
            let k = k.clamp(16, 1024);
            let pool: Vec<Vec<u16>> = (0..k).map(|_| gen(&mut r, set, n)).collect();
            let out_b: usize = pool.iter().map(|u| String::from_utf16_lossy(u).len()).sum::<usize>() / k;
            let mut dst = vec![0u8; 3 * n + 64];
            // Correctness before timing: the three arms agree on this pool.
            for u in &pool {
                let want = String::from_utf16_lossy(u).into_bytes();
                for arm in 0..narms {
                    let rc = unsafe { call(arm, new_tc, old_tc, base_tc, u, &mut dst) };
                    assert!(rc == want.len() as i32 && dst[..want.len()] == want[..], "arm {arm} wrong on {} n={n}", set.name());
                }
            }
            // Calibrate on the old arm (the slowest on valid input).
            let t = Instant::now();
            let mut reps = 0usize;
            while t.elapsed().as_secs_f64() * 1e3 < ms / 4.0 {
                for u in &pool {
                    black_box(unsafe { call(1, new_tc, old_tc, base_tc, u, &mut dst) });
                }
                reps += 1;
            }
            let reps = (reps * 4).max(1);
            let mut ns = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
            for round in 0..rounds {
                for j in 0..narms {
                    let arm = (round + j) % narms;
                    let t = Instant::now();
                    for _ in 0..reps {
                        for u in &pool {
                            black_box(unsafe { call(arm, new_tc, old_tc, base_tc, black_box(u), &mut dst) });
                        }
                    }
                    ns[arm].push(t.elapsed().as_nanos() as f64 / (reps * k) as f64);
                }
            }
            let mut ratio: Vec<f64> = (0..rounds).map(|i| ns[0][i] / ns[1][i]).collect();
            let ratio = med(&mut ratio);
            for (arm, name) in [(3, "pre-D19-core"), (1, "old-scalar"), (0, "new-simdutf"), (2, "export")] {
                if arm >= narms {
                    continue;
                }
                let mut v = ns[arm].clone();
                let lo = v.iter().cloned().fold(f64::MAX, f64::min);
                let hi = v.iter().cloned().fold(0.0, f64::max);
                let m = med(&mut v);
                let gb = |x: f64| (2 * n) as f64 / x;
                println!(
                    "{} | {} | {} | {} | {} | {:.1} [{:.1}..{:.1}] | {:.2} [{:.2}..{:.2}]",
                    set.name(), n, k, out_b, name, m, lo, hi, gb(m), gb(hi), gb(lo)
                );
            }
            println!("{} | {} | | | new/old-scalar (median of per-round ratios) | {:.3} | ", set.name(), n, ratio);
            if narms == 4 {
                let mut c: Vec<f64> = (0..rounds).map(|i| ns[0][i] / ns[3][i]).collect();
                println!("{} | {} | | | new/pre-D19-core (median of per-round ratios) | {:.3} | ", set.name(), n, med(&mut c));
                let mut c: Vec<f64> = (0..rounds).map(|i| ns[3][i] / ns[1][i]).collect();
                println!("{} | {} | | | pre-D19-core/old-scalar (control) | {:.3} | ", set.name(), n, med(&mut c));
            }
        }
    }
}

#[inline(always)]
unsafe fn call(arm: usize, new_tc: ak_transcode_fn, old_tc: ak_transcode_fn, base_tc: ak_transcode_fn, u: &[u16], dst: &mut [u8]) -> i32 {
    match arm {
        3 => base_tc(u.as_ptr() as *const c_void, u.len(), dst.as_mut_ptr(), dst.len() as i32, no_grow, core::ptr::null_mut()),
        0 => new_tc(u.as_ptr() as *const c_void, u.len(), dst.as_mut_ptr(), dst.len() as i32, no_grow, core::ptr::null_mut()),
        1 => old_tc(u.as_ptr() as *const c_void, u.len(), dst.as_mut_ptr(), dst.len() as i32, no_grow, core::ptr::null_mut()),
        _ => ak_utf16_to_utf8(u.as_ptr(), u.len(), dst.as_mut_ptr(), dst.len()),
    }
}
