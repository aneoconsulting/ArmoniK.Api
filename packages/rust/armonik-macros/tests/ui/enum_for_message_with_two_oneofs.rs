include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.TwoOneofs")]
pub enum TwoOneofs {
    A(String),
    B(String),
}
