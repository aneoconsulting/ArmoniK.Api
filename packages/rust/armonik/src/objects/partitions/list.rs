use super::{filter, Field, Raw, Sort};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.partitions.ListPartitionsRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub page: i32,
    pub page_size: i32,
}

impl Request {
    /// A first page of 100 partitions, sorted ascending on the partition id,
    /// with no filter. `Default::default()` is the proto zero value, like every
    /// armonik type, so a page size of 0 and a sort field naming no field at
    /// all: this names one.
    pub fn recommended() -> Self {
        Self {
            sort: Sort::ascending(Field::Id),
            page_size: 100,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    /// See [`crate::tasks::list`]'s counterpart: the zero value of a field enum names no field.
    #[test]
    fn recommended_names_a_real_sort_field() {
        let field = super::Request::recommended().sort.field;
        assert_ne!(i32::from(field), 0);
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.partitions.ListPartitionsResponse")]
pub struct Response {
    pub partitions: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
