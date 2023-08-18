pub mod filter;
pub mod list;

mod field;
mod raw;

pub use field::Field;
pub use raw::Raw;

pub type Sort = super::SortMany<Field>;

super::impl_convert!(
    struct Sort = crate::api::v3::applications::list_applications_request::Sort { direction = enum direction, list fields }
);
