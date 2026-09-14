#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

pub struct Client;

// The RPC name is checked against the service. The impl block survives the error: `probe` is still
// a method afterwards, so a mistake here does not take the rest of the block out of the IDE.
#[armonik_macros::client]
#[armonik(service = "fixture.Fixture")]
impl Client {
    #[armonik(rpc = "Nope")]
    pub fn probe(&self) {}
}

const _: () = {
    // Proof the block survived: this would not resolve if the expansion were only `compile_error!`.
    let _ = Client::probe;
};
