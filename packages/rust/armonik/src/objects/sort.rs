#[armonik_macros::enumeration("armonik.api.grpc.v1.sort_direction.SortDirection")]
#[derive(Debug, Clone, Copy, Default)]
pub enum SortDirection {
    #[default]
    Unspecified,
    Asc,
    Desc,
    /// Unknown to this crate version.
    Unknown(UnknownSortDirection),
}

/// Sort on a single field; stands for the per-service `Sort` messages, whose
/// concrete instantiations are validated by the differential harness.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sort<T> {
    #[armonik(tag = 1)]
    pub field: T,
    #[armonik(tag = 2)]
    pub direction: SortDirection,
}

impl<T> Sort<T> {
    /// Sort on `field`, smallest first.
    pub fn ascending(field: T) -> Self {
        Self {
            field,
            direction: SortDirection::Asc,
        }
    }

    /// Sort on `field`, largest first.
    pub fn descending(field: T) -> Self {
        Self {
            field,
            direction: SortDirection::Desc,
        }
    }
}

/// Sort on several fields; stands for the per-service `Sort` messages with
/// repeated fields.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SortMany<T> {
    #[armonik(tag = 1)]
    pub fields: Vec<T>,
    #[armonik(tag = 2)]
    pub direction: SortDirection,
}

impl<T> SortMany<T> {
    /// Sort on `fields`, smallest first.
    pub fn ascending(fields: impl IntoIterator<Item = T>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
            direction: SortDirection::Asc,
        }
    }

    /// Sort on `fields`, largest first.
    pub fn descending(fields: impl IntoIterator<Item = T>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
            direction: SortDirection::Desc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SortDirection;

    /// An enum whose zero value has a named variant: the catch-all covers only the unknown values,
    /// which sort by what they hold, so after every named one here.
    #[test]
    fn ordering_follows_the_proto_values() {
        assert!(SortDirection::Unspecified < SortDirection::Asc);
        assert!(SortDirection::Asc < SortDirection::Desc);
        assert!(SortDirection::Desc < SortDirection::from(77));
    }
}
