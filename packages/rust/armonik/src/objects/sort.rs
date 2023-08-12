use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum SortDirection {
    Unspecified = 0, /* Unspecified. Do not use. */
    #[default]
    Asc = 1, /* Ascending. */
    Desc = 2,        /* Descending. */
}

impl From<i32> for SortDirection {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Asc,
            2 => Self::Desc,
            _ => Self::Unspecified,
        }
    }
}

impl From<SortDirection> for v3::sort_direction::SortDirection {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::Unspecified => Self::Unspecified,
            SortDirection::Asc => Self::Asc,
            SortDirection::Desc => Self::Desc,
        }
    }
}

impl From<v3::sort_direction::SortDirection> for SortDirection {
    fn from(value: v3::sort_direction::SortDirection) -> Self {
        match value {
            v3::sort_direction::SortDirection::Unspecified => Self::Unspecified,
            v3::sort_direction::SortDirection::Asc => Self::Asc,
            v3::sort_direction::SortDirection::Desc => Self::Desc,
        }
    }
}

super::impl_convert!(SortDirection : Option<v3::sort_direction::SortDirection>);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sort<T> {
    pub field: T,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SortMany<T> {
    pub fields: Vec<T>,
    pub direction: SortDirection,
}
