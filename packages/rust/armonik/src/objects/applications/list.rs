use super::{filter, Field, Raw, Sort};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.applications.ListApplicationsRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub page: i32,
    pub page_size: i32,
}

impl Request {
    /// A first page of 100 applications, sorted ascending on the application
    /// name, with no filter. `Default::default()` is the proto zero value, like
    /// every armonik type, so a page size of 0 and an empty sort naming no
    /// field at all: this names one.
    pub fn recommended() -> Self {
        Self {
            sort: Sort::ascending([Field::Name]),
            page_size: 100,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    /// See [`crate::tasks::list`]'s counterpart. This one sorts on several fields, so the empty
    /// list is the shape that names no field, and the zero value is still not one.
    #[test]
    fn recommended_names_a_real_sort_field() {
        let fields = super::Request::recommended().sort.fields;
        assert!(!fields.is_empty(), "recommended() names a sort field");
        for field in fields {
            assert_ne!(i32::from(field), 0);
        }
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.applications.ListApplicationsResponse")]
pub struct Response {
    pub applications: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
