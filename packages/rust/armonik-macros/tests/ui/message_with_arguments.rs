#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// One error, and it is the real one. What that costs is not visible here: rustc aborts after macro
// expansion once expansion has reported an error, so a one-file crate never reaches the errors a
// deleted item would cause. Measured in `armonik` instead, by giving `objects/tasks/summary.rs` an
// argument: three errors without `item::salvage`, two of them `E0432: unresolved import
// super::Summary` suggesting an unrelated item of the same name, and one with it.
#[armonik_macros::message(fixture.Simple)]
#[derive(Debug, Default)]
pub struct Simple {
    pub name: String,
    pub count: i32,
}
