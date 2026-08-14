#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

pub struct Client;

// No `service` key, so nothing can be looked up. The block still comes back out.
#[armonik_macros::client]
impl Client {
    #[armonik(rpc = "Get")]
    pub fn probe(&self) {}
}

const _: () = {
    let _ = Client::probe;
};
