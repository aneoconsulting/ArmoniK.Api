//! M2: `TaskDetailed` in `ListTasksDetailedResponse`. The shape the control plane actually
//! moves, and the first one in this slice that is not a leaf: repeated strings, a
//! `map<string, string>` inside an inlined child, and nesting to depth 3.
//!
//! The element type fails ABI v1 7.2's batching predicate, so where M1 handed a thousand
//! rows over in a handful of runs, this one costs two calls per element plus one run per
//! repeated field the element carries.

use facade::ListTasksDetailedResponse as Facade;
use prost::Message;
use shapes_prost::shapes as p;

pub const P2_1: &str = "P2.1";
pub const P2_2: &str = "P2.2";
pub const P2_3: &str = "P2.3";
pub const P2_4: &str = "P2.4";
pub const P2_5: &str = "P2.5";
pub const ALL: [&str; 5] = [P2_1, P2_2, P2_3, P2_4, P2_5];

pub mod prost_arm {
    use super::*;
    use stage1_validate::build::{self, Mode};

    /// Built by the HAND-WRITTEN builder of `stage1-validate`, so the prost arm's object
    /// graph is independent of the generated one every facade arm uses.
    pub fn value(pid: &str) -> p::ListTasksDetailedResponse {
        match pid {
            super::P2_1 => build::list_tasks(1, Mode::Full, &[3]),
            super::P2_2 => build::list_tasks(500, Mode::Full, &[3]),
            super::P2_3 => build::list_tasks(125, Mode::Full, &[30]),
            super::P2_4 => build::list_tasks(80, Mode::Full, &[3, 150]),
            super::P2_5 => build::list_tasks(20, Mode::HalfAbsent, &[3]),
            _ => unimplemented!("{pid}"),
        }
    }

    pub fn encode(v: &p::ListTasksDetailedResponse) -> Vec<u8> {
        v.encode_to_vec()
    }

    pub fn decode(b: &[u8]) -> p::ListTasksDetailedResponse {
        p::ListTasksDetailedResponse::decode(b).unwrap()
    }
}

pub mod armonik_arm {
    use super::*;
    use facade::build as fb;

    pub fn value(pid: &str) -> Facade {
        match pid {
            super::P2_1 => fb::payload_p2_1(),
            super::P2_2 => fb::payload_p2_2(),
            super::P2_3 => fb::payload_p2_3(),
            super::P2_4 => fb::payload_p2_4(),
            super::P2_5 => fb::payload_p2_5(),
            _ => unimplemented!("{pid}"),
        }
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
        cn::encode_list_tasks_detailed_response(v)
    }

    pub fn decode(b: &[u8]) -> Facade {
        cn::decode_list_tasks_detailed_response(b).expect("core-native decode M2")
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
        binding::encode_into_list_tasks_detailed_response(c.enc, v, &c.tcs)
            .expect("core-ffi encode M2");
        unsafe { binding::encoded(c.enc) }
    }

    pub fn encode(c: &Ctx, v: &Facade) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }

    pub fn decode(c: &Ctx, b: &[u8]) -> Facade {
        binding::decode_with_list_tasks_detailed_response(c.dec, b).expect("core-ffi decode M2")
    }
}

/// Two added arms, not payloads: `design/SHAPES.md` is not changed, and a slice may add an
/// arm. They exist to isolate what ABI v1 open decision 5 actually costs.
///
/// P2.4 is 80 elements alternating 3 and 150 repeated strings, so element bodies alternate
/// across a varint length boundary and a per-site learned width is wrong on every element by
/// construction. P2.4a is 80 elements all at 3, P2.4b is 80 elements all at 150. By
/// construction `bytes(P2.4) == (bytes(P2.4a) + bytes(P2.4b)) / 2` and the element count,
/// the field count and the string count all match too, so
///
///     t(P2.4) - (t(P2.4a) + t(P2.4b)) / 2
///
/// is the length-prefix thrashing and nothing else, measured in one process. Neither has a
/// manifest row, so their correctness check is byte identity against the prost arm rather
/// than against a validated hash.
pub const P2_4A: &str = "P2.4a";
pub const P2_4B: &str = "P2.4b";
pub const ADDED: [&str; 2] = [P2_4A, P2_4B];

pub mod added {
    use super::*;
    use facade::build::{build_task_detailed, Mode};
    use stage1_validate::build as sb;

    fn reps(pid: &str) -> i64 {
        match pid {
            super::P2_4A => 3,
            super::P2_4B => 150,
            _ => unimplemented!("{pid}"),
        }
    }

    pub fn prost_value(pid: &str) -> p::ListTasksDetailedResponse {
        sb::list_tasks(80, sb::Mode::Full, &[reps(pid)])
    }

    pub fn facade_value(pid: &str) -> Facade {
        let r = reps(pid);
        Facade {
            tasks: (0..80i64)
                .map(|j| build_task_detailed("TaskDetailed", j, Mode::Full, r, 0))
                .collect(),
            page: 1,
            total: 80,
        }
    }
}

/// ABI v1 open decision 9 candidate on the shape the control plane actually moves.
/// See `arms::core_ffi_zeroed` for what it is and what trade it reverses.
pub mod core_ffi_zeroed {
    use super::*;
    use crate::arms::core_ffi_arm::Ctx;
    use crate::generated::binding;

    pub fn encode_into<'a>(c: &'a Ctx, v: &Facade) -> &'a [u8] {
        binding::encode_into_list_tasks_detailed_response_zeroed(c.enc, v, &c.tcs)
            .expect("core-ffi-zeroed encode");
        unsafe { binding::encoded(c.enc) }
    }

    pub fn encode(c: &Ctx, v: &Facade) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }
}
