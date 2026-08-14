use super::{filter, Field, Raw, RawField, Sort};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.sessions.ListSessionsRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub with_task_options: bool,
    pub page: i32,
    pub page_size: i32,
}

impl Request {
    /// A first page of 100 sessions, sorted ascending on the session id, with
    /// no filter. `Default::default()` is the proto zero value, like every
    /// armonik type, so a page size of 0 and a sort field naming no field at
    /// all: this names one.
    pub fn recommended() -> Self {
        Self {
            sort: Sort::ascending(Field::Raw(RawField::SessionId)),
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
        let super::Field::Raw(field) = super::Request::recommended().sort.field else {
            panic!("recommended() sorts on a session raw field");
        };
        assert_ne!(i32::from(field), 0);
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.sessions.ListSessionsResponse")]
pub struct Response {
    pub sessions: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
