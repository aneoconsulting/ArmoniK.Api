#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

pub struct Client;

#[armonik_macros::client]
#[armonik(service = "fixture.Nope")]
impl Client {
    #[armonik(rpc = "Get")]
    pub fn probe(&self) {}
}

const _: () = {
    let _ = Client::probe;
};
