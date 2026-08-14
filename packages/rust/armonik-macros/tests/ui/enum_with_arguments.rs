#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// The enumeration half of `message_with_arguments`, which is a separate case because the salvage
// is: an enumeration's re-emitted item names a catch-all payload struct that only the expansion
// emits, so its stub carries that struct and, under `--all-features`, the serde derive the
// re-emitted enum's own `derive(Deserialize)` needs from it.
#[armonik_macros::enumeration(fixture.Colour)]
#[derive(Debug, Clone, Copy)]
pub enum Colour {
    Red,
    Green,
    Unknown(UnknownColour),
}
