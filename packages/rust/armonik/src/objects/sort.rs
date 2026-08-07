#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.sort_direction.SortDirection")]
pub enum SortDirection {
    #[default]
    Unspecified,
    Asc,
    Desc,
    /// Unknown to this crate version.
    Other(OtherSortDirection),
}

/// Sort on a single field; stands for the per-service `Sort` messages, whose
/// concrete instantiations are validated by the differential harness.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(generic)]
pub struct Sort<T> {
    #[armonik(tag = 1)]
    pub field: T,
    #[armonik(tag = 2)]
    pub direction: SortDirection,
}

/// Sort on several fields; stands for the per-service `Sort` messages with
/// repeated fields.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(generic)]
pub struct SortMany<T> {
    #[armonik(tag = 1)]
    pub fields: Vec<T>,
    #[armonik(tag = 2)]
    pub direction: SortDirection,
}

#[cfg(test)]
mod tests {
    use super::SortDirection;

    /// An enum whose zero value has a named variant: the catch-all covers only the unknown values,
    /// and still sorts before every named one.
    #[test]
    fn ordering_follows_the_proto_values() {
        assert!(SortDirection::Unspecified < SortDirection::Asc);
        assert!(SortDirection::Asc < SortDirection::Desc);
        assert!(SortDirection::from(77) < SortDirection::Unspecified);
    }
}
