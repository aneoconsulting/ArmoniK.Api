#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

pub struct Client;

// Every method has to say which RPC it stands for, or the coverage check cannot see it.
#[armonik_macros::client]
#[armonik(service = "fixture.Fixture")]
impl Client {
    pub fn probe(&self) {}
}

const _: () = {
    let _ = Client::probe;
};
