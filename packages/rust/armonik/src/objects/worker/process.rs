use super::super::Output;

use crate::api::v3;

pub struct Response {
    pub communication_token: String,
    pub output: Output,
}

super::super::impl_convert!(
    struct Response = v3::worker::ProcessReply {
        communication_token,
        output = option output,
    }
);
