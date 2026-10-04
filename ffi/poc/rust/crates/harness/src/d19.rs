//! D19 (owner, 2026-10-04): the core's UTF-16 transcoder on simdutf, and the additive UTF
//! conversion exports. What the differential (`bin/tc16_diff`) and the instrumentation
//! (`bin/tc16_bench`) share: the exports' declarations, the input generators and a way to
//! drive a transcoder through the C ABI with a chosen capacity and grow.
//!
//! The exports are declared HERE, by this host, as ABI v1 asks of `ak_utf8_check`'s
//! successors: no generated or shared header carries them.
use ak_abi::*;
use core::ffi::c_void;

extern "C" {
    /// The pre-D19 scalar transcoder, exported as the oracle and the control.
    pub fn ak_tc_utf16_scalar() -> ak_transcode_fn;
    pub fn ak_utf16_to_utf8(src: *const u16, len: usize, dst: *mut u8, cap: usize) -> i32;
    pub fn ak_utf16_utf8_len(src: *const u16, len: usize) -> i32;
    pub fn ak_utf8_to_utf16(src: *const u8, len: usize, dst: *mut u16, cap: usize) -> i32;
    pub fn ak_utf8_utf16_len(src: *const u8, len: usize) -> i32;
    pub fn ak_utf8_validate(src: *const u8, len: usize) -> i32;
    pub fn ak_utf16_validate(src: *const u16, len: usize) -> i32;
    pub fn ak_utf8_check(src: *const u8, len: usize) -> i32;
}

/// SplitMix64: deterministic, seeded per run and printed.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
}

/// The content sets. The first five are valid UTF-16 (the timed sets); the last three
/// carry lone surrogates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Set {
    Ascii,
    Latin1,
    /// BMP above U+07FF, mostly CJK (3 UTF-8 bytes per unit).
    Wide,
    /// Astral code points only (surrogate pairs; emoji range mostly).
    Astral,
    /// A valid mix of all four classes.
    Mixed,
    /// A valid mix with lone surrogates injected at the start, middle and end.
    Lone,
    /// Every u16 value uniformly: paired, unpaired, reversed pairs.
    Random16,
    /// Surrogates only, high and low at random: mostly unpaired.
    Surrogates,
}
pub const VALID_SETS: [Set; 5] = [Set::Ascii, Set::Latin1, Set::Wide, Set::Astral, Set::Mixed];
pub const ALL_SETS: [Set; 8] =
    [Set::Ascii, Set::Latin1, Set::Wide, Set::Astral, Set::Mixed, Set::Lone, Set::Random16, Set::Surrogates];

impl Set {
    pub fn name(self) -> &'static str {
        match self {
            Set::Ascii => "ascii",
            Set::Latin1 => "latin1",
            Set::Wide => "bmp-wide",
            Set::Astral => "astral",
            Set::Mixed => "mixed",
            Set::Lone => "lone-surrogates",
            Set::Random16 => "random-u16",
            Set::Surrogates => "surrogates-only",
        }
    }
}

fn push_cp(v: &mut Vec<u16>, cp: u32) {
    let mut b = [0u16; 2];
    v.extend_from_slice(char::from_u32(cp).unwrap().encode_utf16(&mut b));
}

fn valid_cp(r: &mut Rng, class: u64) -> u32 {
    match class {
        0 => r.below(0x80) as u32,
        1 => 0x80 + r.below(0x80) as u32,
        2 => {
            // CJK unified ideographs most of the time, any non-surrogate BMP otherwise.
            if r.below(4) != 0 {
                0x4E00 + r.below(0x5200) as u32
            } else {
                loop {
                    let c = 0x800 + r.below(0x10000 - 0x800) as u32;
                    if !(0xD800..0xE000).contains(&c) {
                        break c;
                    }
                }
            }
        }
        _ => {
            if r.below(4) != 0 {
                0x1F300 + r.below(0x300) as u32
            } else {
                0x10000 + r.below(0x100000) as u32
            }
        }
    }
}

/// `n` UTF-16 code units of the set (an astral set may end one unit short of a pair; it
/// stays valid by ending on a BMP unit).
pub fn gen(r: &mut Rng, set: Set, n: usize) -> Vec<u16> {
    let mut v = Vec::with_capacity(n + 1);
    match set {
        Set::Ascii => v.extend((0..n).map(|_| r.below(0x80) as u16)),
        Set::Latin1 => v.extend((0..n).map(|_| r.below(0x100) as u16)),
        Set::Wide | Set::Astral | Set::Mixed | Set::Lone => {
            while v.len() < n {
                let class = match set {
                    Set::Wide => 2,
                    Set::Astral => 3,
                    _ => r.below(4),
                };
                let cp = valid_cp(r, class);
                if cp > 0xFFFF && v.len() + 2 > n {
                    push_cp(&mut v, 0x41);
                } else {
                    push_cp(&mut v, cp);
                }
            }
            if set == Set::Lone && n > 0 {
                let lone = |r: &mut Rng| if r.below(2) == 0 { 0xD800 + r.below(0x400) as u16 } else { 0xDC00 + r.below(0x400) as u16 };
                // start, middle, end, and one more anywhere; each may split a pair.
                let k = 1 + r.below(4);
                for j in 0..k {
                    let at = match j {
                        0 => 0,
                        1 => n / 2,
                        2 => n - 1,
                        _ => r.below(n as u64) as usize,
                    };
                    v[at] = lone(r);
                }
            }
        }
        Set::Random16 => v.extend((0..n).map(|_| r.next() as u16)),
        Set::Surrogates => v.extend((0..n).map(|_| 0xD800 + r.below(0x800) as u16)),
    }
    v.truncate(n);
    v
}

/// A Vec-backed transcoder sink. `mode` sets what `grow` does.
#[derive(Clone, Copy, Debug)]
pub enum Grow {
    /// The core's own (`Enc::grow` is `Vec::reserve`): capacity becomes max(2 x cap, want).
    Geometric,
    /// Gives `min(want, limit)` bytes.
    Limit(usize),
    /// Refuses: returns AK_ERR_CAPACITY, leaves dst and cap.
    Refuse,
}
pub struct Sink {
    pub buf: Vec<u8>,
    pub cap: usize,
    pub mode: Grow,
    pub grows: u32,
    pub wants: Vec<i32>,
}
/// Bytes of canary past the capacity handed to the transcoder.
pub const CANARY: usize = 64;
pub const CANARY_BYTE: u8 = 0xA5;

unsafe extern "C" fn sink_grow(sink: *mut c_void, want: i32, dst: *mut *mut u8, cap: *mut i32) -> i32 {
    let s = &mut *(sink as *mut Sink);
    s.grows += 1;
    s.wants.push(want);
    let want = want.max(0) as usize;
    let give = match s.mode {
        Grow::Geometric => want.max(2 * s.cap),
        Grow::Limit(l) => want.min(l),
        Grow::Refuse => return AK_ERR_CAPACITY,
    };
    s.cap = give;
    s.buf = vec![CANARY_BYTE; give + CANARY];
    *dst = s.buf.as_mut_ptr();
    *cap = give as i32;
    AK_OK
}

/// What one transcoder call produced: its return, the bytes it reports, and whether it
/// wrote into the canary past the capacity it was last given.
#[derive(Debug, PartialEq, Eq)]
pub struct Out {
    pub rc: i32,
    pub bytes: Vec<u8>,
    pub canary_ok: bool,
}

/// One call of `tc` over `u` with an initial capacity `cap` and the given grow.
pub unsafe fn run_tc(tc: ak_transcode_fn, u: &[u16], cap: usize, mode: Grow) -> (Out, Sink) {
    let mut sink = Sink { buf: vec![CANARY_BYTE; cap + CANARY], cap, mode, grows: 0, wants: Vec::new() };
    let dst = sink.buf.as_mut_ptr();
    let src = if u.is_empty() { core::ptr::null() } else { u.as_ptr() as *const c_void };
    let rc = tc(src, u.len(), dst, cap as i32, sink_grow, &mut sink as *mut Sink as *mut c_void);
    let canary_ok = sink.buf[sink.cap..].iter().all(|&b| b == CANARY_BYTE);
    let bytes = if rc > 0 { sink.buf[..rc as usize].to_vec() } else { Vec::new() };
    (Out { rc, bytes, canary_ok }, sink)
}

/// A PLANTED transcoder (the differential's positive control): the pre-D19 algorithm with
/// `?` for a lone surrogate instead of U+FFFD. Must differ on every set with one.
pub unsafe extern "C" fn planted_question_mark(
    src: *const c_void,
    len: usize,
    mut dst: *mut u8,
    mut cap: i32,
    grow: ak_grow_fn,
    sink: *mut c_void,
) -> i32 {
    if len == 0 {
        return 0;
    }
    let u = core::slice::from_raw_parts(src as *const u16, len);
    let s: String = char::decode_utf16(u.iter().copied()).map(|r| r.unwrap_or('?')).collect();
    let need = s.len();
    if need as i64 > cap as i64 {
        let rc = grow(sink, need as i32, &mut dst, &mut cap);
        if rc < 0 {
            return rc;
        }
        if need as i64 > cap as i64 {
            return AK_ERR_CAPACITY;
        }
    }
    core::ptr::copy_nonoverlapping(s.as_ptr(), dst, need);
    need as i32
}

/// A PLANTED transcoder: correct bytes, then one stray byte written just past `cap`. The
/// canary check must catch it on every call that fills its buffer.
pub unsafe extern "C" fn planted_overrun(
    src: *const c_void,
    len: usize,
    mut dst: *mut u8,
    mut cap: i32,
    grow: ak_grow_fn,
    sink: *mut c_void,
) -> i32 {
    if len == 0 {
        return 0;
    }
    let u = core::slice::from_raw_parts(src as *const u16, len);
    let s = String::from_utf16_lossy(u);
    let need = s.len();
    if need as i64 > cap as i64 {
        let rc = grow(sink, need as i32, &mut dst, &mut cap);
        if rc < 0 {
            return rc;
        }
        if need as i64 > cap as i64 {
            return AK_ERR_CAPACITY;
        }
    }
    core::ptr::copy_nonoverlapping(s.as_ptr(), dst, need);
    *dst.add(cap as usize) = 0;
    need as i32
}

extern "C" {
    fn dlopen(file: *const core::ffi::c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const core::ffi::c_char) -> *mut c_void;
    fn dlerror() -> *const core::ffi::c_char;
}

/// The PRE-D19 core binary's `ak_tc_utf16`, when `AK_D19_BASE_CORE` names its
/// libak_core.so: loaded beside this process's core with RTLD_LOCAL (its symbols stay out of
/// the global scope), so the transcoder of the commit before D19 runs in the same process
/// as the new one. `tc_utf16` there calls only functions internal to that object and the
/// grow it is handed, so nothing in it resolves to this process's core. The path and the
/// symbol address are printed by the caller.
pub fn base_core_tc16() -> Option<(String, ak_transcode_fn)> {
    let path = std::env::var("AK_D19_BASE_CORE").ok().filter(|p| !p.is_empty())?;
    let c = std::ffi::CString::new(path.clone()).unwrap();
    unsafe {
        // RTLD_NOW | RTLD_LOCAL
        let h = dlopen(c.as_ptr(), 2);
        if h.is_null() {
            panic!("dlopen {path}: {:?}", std::ffi::CStr::from_ptr(dlerror()));
        }
        let f = dlsym(h, c"ak_tc_utf16".as_ptr());
        assert!(!f.is_null(), "{path}: no ak_tc_utf16");
        let get: unsafe extern "C" fn() -> ak_transcode_fn = core::mem::transmute(f);
        assert!(dlsym(h, c"ak_tc_utf16_scalar".as_ptr()).is_null(), "{path} is not a pre-D19 core (it has ak_tc_utf16_scalar)");
        Some((path, get()))
    }
}
