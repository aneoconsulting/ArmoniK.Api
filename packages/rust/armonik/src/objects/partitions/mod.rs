pub mod filter;
pub mod get;
pub mod list;

mod field;
mod raw;

pub use field::Field;
pub use raw::Raw;

pub type Sort = super::Sort<Field>;

super::super::impl_convert!(
    struct Sort = crate::api::v3::partitions::list_partitions_request::Sort {
        field = option field,
        direction = enum direction,
    }
);
