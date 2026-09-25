//! CAMPAIGN.md requirement 19: boundary-call counts of every core-ffi case the codec suite
//! times -- every input (the 16 payloads, the content sets, the `U-*` rows), encode and push
//! and pull decode, drop and retain -- from a COUNTING build (`--features count`), counted
//! in the core (forward, reverse). Machine-independent: the runner compares the output with
//! the committed `gen/crossings.txt` and stops on any difference.
//!
//! Each count is taken on a warm context (the case run once before, as the timed loop is).

use ak_abi::*;
use campaign::*;
use harness::arms::core_ffi_arm::Ctx;

struct Count<'a> {
    ctx: &'a Ctx,
    inp: &'a Input,
    out: &'a mut Vec<String>,
}

fn enc(ctx: &Ctx) -> (u64, u64) {
    let mut c = AkCounters::default();
    unsafe { ak_enc_counters(ctx.enc, &mut c) };
    (c.forward, c.reverse)
}
fn dec(ctx: &Ctx) -> (u64, u64) {
    let mut c = AkCounters::default();
    unsafe { ak_dec_counters(ctx.dec, &mut c) };
    (c.forward, c.reverse)
}

impl Visit for Count<'_> {
    fn visit<R: Ops>(&mut self) {
        let (c, w) = (self.ctx, &self.inp.bytes[..]);
        for (m, retain) in [("drop", false), ("retain", true)] {
            if self.inp.encode {
                let v = R::n_decode(w, retain && self.inp.unknown_row).expect("value");
                R::f_encode(c, &v, retain).expect("encode");
                unsafe { ak_enc_counters_reset(c.enc) };
                R::f_encode(c, &v, retain).expect("encode");
                let (f, r) = enc(c);
                self.out.push(format!("{:<48} {:<12} {:<6} {:>8} {:>8}", self.inp.id, "encode", m, f, r));
            }
            R::f_decode(c, w, retain).expect("decode");
            unsafe { ak_dec_counters_reset(c.dec) };
            R::f_decode(c, w, retain).expect("decode");
            let (f, r) = dec(c);
            self.out.push(format!("{:<48} {:<12} {:<6} {:>8} {:>8}", self.inp.id, "decode", m, f, r));
            let mut toks = Vec::new();
            R::f_pull(c, w, retain, &mut toks).expect("pull");
            unsafe { ak_dec_counters_reset(c.dec) };
            R::f_pull(c, w, retain, &mut toks).expect("pull");
            let (f, r) = dec(c);
            self.out.push(format!("{:<48} {:<12} {:<6} {:>8} {:>8}", self.inp.id, "decode-pull", m, f, r));
        }
    }
}

fn main() {
    let ctx = Ctx::new();
    let mut out = Vec::new();
    for inp in inputs(&[]) {
        campaign::generated::roots::with_root(&inp.root, &mut Count { ctx: &ctx, inp: &inp, out: &mut out });
    }
    // A build without counters reads zero everywhere: refuse rather than write zeros.
    assert!(out.iter().any(|l| !l.ends_with("        0        0")), "not a counting build (--features count)");
    println!("# input                                            direction    mode    forward  reverse");
    for l in out {
        println!("{l}");
    }
}
