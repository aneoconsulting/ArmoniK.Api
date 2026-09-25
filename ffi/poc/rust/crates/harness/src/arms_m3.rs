//! M3: `Probe` in `ListProbeResponse`. A oneof of five members including a payload-free
//! one, and three `optional` scalars. Both shapes are unmeasured on .NET, so these numbers
//! will be read across by the C# slice.
//!
//! Small on purpose: the value of M3 is shape coverage, not another timing table.

use facade::ListProbeResponse as Facade;
use prost::Message;
use shapes_prost::shapes as p;

pub const P3_1: &str = "P3.1";
pub const ALL: [&str; 1] = [P3_1];

pub mod prost_arm {
    use super::*;
    use stage1_validate::build;

    pub fn value(_pid: &str) -> p::ListProbeResponse {
        build::list_probes(200)
    }
    pub fn encode(v: &p::ListProbeResponse) -> Vec<u8> {
        v.encode_to_vec()
    }
    pub fn decode(b: &[u8]) -> p::ListProbeResponse {
        p::ListProbeResponse::decode(b).unwrap()
    }
}

pub mod armonik_arm {
    use super::*;
    pub fn value(_pid: &str) -> Facade {
        facade::build::payload_p3_1()
    }
    pub fn encode(v: &Facade) -> Vec<u8> {
        v.encode_to_vec()
    }
    pub fn decode(b: &[u8]) -> Facade {
        Facade::decode(b).unwrap()
    }
}

pub mod core_native_arm {
    use super::*;
    use facade::generated::core_native as cn;
    pub fn value(pid: &str) -> Facade {
        super::armonik_arm::value(pid)
    }
    pub fn encode(v: &Facade) -> Vec<u8> {
        cn::encode_list_probe_response(v)
    }
    pub fn decode(b: &[u8]) -> Facade {
        cn::decode_list_probe_response(b).expect("core-native decode M3")
    }
}

pub mod core_ffi_arm {
    use super::*;
    use crate::arms::core_ffi_arm::Ctx;
    use crate::generated::binding;
    pub fn value(pid: &str) -> Facade {
        super::armonik_arm::value(pid)
    }
    pub fn encode_into<'a>(c: &'a Ctx, v: &Facade) -> &'a [u8] {
        binding::encode_into_list_probe_response(c.enc, v, &c.tcs).expect("core-ffi encode M3");
        unsafe { binding::encoded(c.enc) }
    }
    pub fn encode(c: &Ctx, v: &Facade) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }
    pub fn decode(c: &Ctx, b: &[u8]) -> Facade {
        binding::decode_with_list_probe_response(c.dec, b).expect("core-ffi decode M3")
    }
}

/// The three cases of explicit presence, which is why the shape is in the payload set at
/// all: a by-value group reports absent and empty identically unless it is designed not to.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct PresenceCensus {
    pub absent: usize,
    pub present_zero: usize,
    pub present_nonzero: usize,
}

impl PresenceCensus {
    pub fn total(&self) -> usize {
        self.absent + self.present_zero + self.present_nonzero
    }
}

/// Census of one `optional` field over a decoded response, so the three cases are three
/// rows rather than one column.
pub fn census(v: &Facade) -> [(&'static str, PresenceCensus); 3] {
    let mut c = [
        ("opt_count (int32)", PresenceCensus::default()),
        ("opt_label (string)", PresenceCensus::default()),
        ("opt_flag  (bool)", PresenceCensus::default()),
    ];
    for e in &v.probes {
        bump(&mut c[0].1, e.opt_count.map(|x| x != 0));
        bump(&mut c[1].1, e.opt_label.as_ref().map(|x| !x.is_empty()));
        bump(&mut c[2].1, e.opt_flag.map(|x| x));
    }
    c
}

fn bump(c: &mut PresenceCensus, v: Option<bool>) {
    match v {
        None => c.absent += 1,
        Some(false) => c.present_zero += 1,
        Some(true) => c.present_nonzero += 1,
    }
}

/// Which oneof member each element carries, counted by variant, including the payload-free
/// one and the `None` case.
pub fn oneof_census(v: &Facade) -> [(&'static str, usize); 6] {
    use facade::ProbeBody::*;
    let mut n = [
        ("as_int (int64)", 0),
        ("as_text (string)", 0),
        ("as_blob (bytes)", 0),
        ("as_stamp (message)", 0),
        ("as_nothing (payload-free)", 0),
        ("none set", 0),
    ];
    for e in &v.probes {
        let i = match &e.body {
            Some(AsInt(_)) => 0,
            Some(AsText(_)) => 1,
            Some(AsBlob(_)) => 2,
            Some(AsStamp(_)) => 3,
            Some(AsNothing(_)) => 4,
            None => 5,
        };
        n[i].1 += 1;
    }
    n
}

/// The zeroed-group arm on M3, which is where it can go WRONG rather than merely slow.
/// Explicit presence and the oneof are the two cases a sparse fill breaks if it tests the
/// VALUE instead of the presence: `Some(0)` and `Some("")` are at their default value and
/// must still be written. The generated sparse fill tests presence for those fields, and
/// this arm is what would catch it if it did not.
pub mod core_ffi_zeroed {
    use super::*;
    use crate::arms::core_ffi_arm::Ctx;
    use crate::generated::binding;

    pub fn encode_into<'a>(c: &'a Ctx, v: &Facade) -> &'a [u8] {
        binding::encode_into_list_probe_response_zeroed(c.enc, v, &c.tcs)
            .expect("core-ffi-zeroed encode M3");
        unsafe { binding::encoded(c.enc) }
    }

    pub fn encode(c: &Ctx, v: &Facade) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }
}

/// The unknown-field bag on M3, which is where the seven unknown-field vectors live.
/// See `arms::core_ffi_unk`.
#[cfg(feature = "unknown-fields")]
pub mod core_ffi_unk {
    use super::*;
    use crate::arms::core_ffi_arm::Ctx;
    use crate::generated::binding;

    pub fn encode_into<'a>(c: &'a Ctx, v: &Facade) -> &'a [u8] {
        binding::encode_into_list_probe_response_unk(c.enc, v, &c.tcs).expect("unk encode M3");
        unsafe { binding::encoded(c.enc) }
    }
    pub fn encode(c: &Ctx, v: &Facade) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }
    pub fn decode(c: &Ctx, b: &[u8]) -> Facade {
        binding::decode_with_list_probe_response_unk(c.dec, b).expect("unk decode M3")
    }
}
