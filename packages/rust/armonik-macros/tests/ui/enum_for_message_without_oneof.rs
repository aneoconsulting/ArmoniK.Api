include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Simple")]
pub enum Simple {
    Name(String),
}
