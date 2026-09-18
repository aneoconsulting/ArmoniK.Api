//! M4 to M7: the adapter site, bulk bytes, packed scalars and interleaved repeated fields.
//!
//! Each is small and each is here for one shape. Classes per `design/SHAPES.md`, and they
//! differ WITHIN M6, which is why the log labels rows and not tables:
//!
//! | shape | class | why |
//! |---|---|---|
//! | M4 `TaskSummary` | real | the only `with` adapter in the Rust crate |
//! | M5 `UploadResultData` | real | the result upload and download path |
//! | M6 `ticks`/`values`/`codes`/`flags` | **CONTROL** | the schema has no packed scalar |
//! | M6 `statuses` | **real** | the schema's 3 packed fields are all enums |
//! | M7 `DualResponse` | **CONTROL** | legal wire no group buffer keyed by type can decode |

use prost::Message;
use shapes_prost::shapes as p;

pub const P4_1: &str = "P4.1";
pub const P5_1: &str = "P5.1";
pub const P5_2: &str = "P5.2";
pub const P5_3: &str = "P5.3";
pub const P5_4: &str = "P5.4";
pub const P6_1: &str = "P6.1";
pub const P7_1: &str = "P7.1";
/// Everything with a builder. P7.1 is absent on purpose: no canonical writer can produce it.
pub const ENCODABLE: [&str; 6] = [P4_1, P5_1, P5_2, P5_3, P5_4, P6_1];

macro_rules! arm4 {
    ($m:ident, $facade:ty, $prost:ty, $pbuild:expr, $fbuild:expr, $enc:ident, $enc_into:ident, $dec:ident) => {
        pub mod $m {
            use super::*;
            pub fn prost_value(pid: &str) -> $prost {
                $pbuild(pid)
            }
            pub fn facade_value(pid: &str) -> $facade {
                $fbuild(pid)
            }
            pub fn prost_encode(v: &$prost) -> Vec<u8> {
                v.encode_to_vec()
            }
            pub fn prost_decode(b: &[u8]) -> $prost {
                <$prost>::decode(b).unwrap()
            }
            pub fn armonik_encode(v: &$facade) -> Vec<u8> {
                v.encode_to_vec()
            }
            pub fn armonik_decode(b: &[u8]) -> $facade {
                <$facade>::decode(b).unwrap()
            }
            pub fn native_encode(v: &$facade) -> Vec<u8> {
                facade::generated::core_native::$enc(v)
            }
            /// Into a buffer the caller reuses, which is what every other arm does. The
            /// allocating form above is for correctness only: timing it against arms that
            /// reuse a buffer measured `core-native` at 1.41 of prost on a 4 MB payload,
            /// which was the harness growing a Vec from 4 KB by doubling, not the codec.
            pub fn native_encode_into(v: &$facade, e: &mut ak_rt::Enc) {
                facade::generated::core_native::$enc_into(v, e);
            }
            pub fn native_decode(b: &[u8]) -> $facade {
                facade::generated::core_native::$dec(b).expect("core-native decode")
            }
        }
    };
}

arm4!(
    m4,
    facade::ListTaskSummaryResponse,
    p::ListTaskSummaryResponse,
    |_p| stage1_validate::build::list_summaries(200),
    |_p| facade::build::payload_p4_1(),
    encode_list_task_summary_response,
    encode_into_list_task_summary_response,
    decode_list_task_summary_response
);

arm4!(
    m6,
    facade::ListMetricsResponse,
    p::ListMetricsResponse,
    |_p| stage1_validate::build::list_metrics(200),
    |_p| facade::build::payload_p6_1(),
    encode_list_metrics_response,
    encode_into_list_metrics_response,
    decode_list_metrics_response
);

/// M5 is parameterised by bulk size, so it does not fit the macro.
pub mod m5 {
    use super::*;

    fn bulk(pid: &str) -> usize {
        match pid {
            super::P5_1 => 36,
            super::P5_2 => 65536,
            super::P5_3 => 1048576,
            super::P5_4 => 4194304,
            _ => unimplemented!("{pid}"),
        }
    }

    pub fn prost_value(pid: &str) -> p::UploadResultDataMessage {
        stage1_validate::build::upload(bulk(pid))
    }
    pub fn facade_value(pid: &str) -> facade::UploadResultDataMessage {
        match pid {
            super::P5_1 => facade::build::payload_p5_1(),
            super::P5_2 => facade::build::payload_p5_2(),
            super::P5_3 => facade::build::payload_p5_3(),
            super::P5_4 => facade::build::payload_p5_4(),
            _ => unimplemented!("{pid}"),
        }
    }
    pub fn prost_encode(v: &p::UploadResultDataMessage) -> Vec<u8> {
        v.encode_to_vec()
    }
    pub fn prost_decode(b: &[u8]) -> p::UploadResultDataMessage {
        p::UploadResultDataMessage::decode(b).unwrap()
    }
    pub fn armonik_encode(v: &facade::UploadResultDataMessage) -> Vec<u8> {
        v.encode_to_vec()
    }
    pub fn armonik_decode(b: &[u8]) -> facade::UploadResultDataMessage {
        facade::UploadResultDataMessage::decode(b).unwrap()
    }
    pub fn native_encode(v: &facade::UploadResultDataMessage) -> Vec<u8> {
        facade::generated::core_native::encode_upload_result_data_message(v)
    }
    pub fn native_encode_into(v: &facade::UploadResultDataMessage, e: &mut ak_rt::Enc) {
        facade::generated::core_native::encode_into_upload_result_data_message(v, e);
    }
    pub fn native_decode(b: &[u8]) -> facade::UploadResultDataMessage {
        facade::generated::core_native::decode_upload_result_data_message(b).expect("m5 decode")
    }
}

/// M7 is DECODE ONLY. No canonical writer can produce its bytes: it interleaves two repeated
/// fields of one type on purpose, and every writer here emits a field contiguously.
pub mod m7 {
    use super::*;

    pub fn prost_decode(b: &[u8]) -> p::DualResponse {
        p::DualResponse::decode(b).unwrap()
    }
    pub fn armonik_decode(b: &[u8]) -> facade::DualResponse {
        facade::DualResponse::decode(b).unwrap()
    }
    pub fn native_decode(b: &[u8]) -> facade::DualResponse {
        facade::generated::core_native::decode_dual_response(b).expect("m7 decode")
    }
    pub fn native_encode(v: &facade::DualResponse) -> Vec<u8> {
        facade::generated::core_native::encode_dual_response(v)
    }
    pub fn armonik_encode(v: &facade::DualResponse) -> Vec<u8> {
        v.encode_to_vec()
    }
}

/// The `core-ffi-rust` arm for M4 to M7, through the generated binding.
pub mod ffi {
    use crate::arms::core_ffi_arm::Ctx;
    use crate::generated::binding as b;

    macro_rules! pair {
        ($enc:ident, $dec:ident, $ty:ty, $e:ident, $d:ident) => {
            pub fn $enc<'a>(c: &'a Ctx, v: &$ty) -> &'a [u8] {
                b::$e(c.enc, v, &c.tcs).expect("core-ffi encode");
                unsafe { b::encoded(c.enc) }
            }
            pub fn $dec(c: &Ctx, x: &[u8]) -> $ty {
                b::$d(c.dec, x).expect("core-ffi decode")
            }
        };
    }

    pair!(
        enc_m4, dec_m4, facade::ListTaskSummaryResponse,
        encode_into_list_task_summary_response, decode_with_list_task_summary_response
    );
    pair!(
        enc_m5, dec_m5, facade::UploadResultDataMessage,
        encode_into_upload_result_data_message, decode_with_upload_result_data_message
    );
    pair!(
        enc_m6, dec_m6, facade::ListMetricsResponse,
        encode_into_list_metrics_response, decode_with_list_metrics_response
    );
    pair!(
        enc_m7, dec_m7, facade::DualResponse,
        encode_into_dual_response, decode_with_dual_response
    );
}
