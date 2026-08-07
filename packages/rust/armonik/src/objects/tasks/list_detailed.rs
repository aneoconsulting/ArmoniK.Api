use super::{filter, Raw, Sort};

/// Shares its wire form (`ListTasksRequest`) with [`super::list::Request`];
/// a distinct type keeps the two RPCs' requests distinct (request types are
/// injective over RPCs).
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub with_errors: bool,
    pub page: i32,
    pub page_size: i32,
}

impl Request {
    /// A first page of 100 tasks, sorted ascending on the default field, with
    /// no filter. `Default::default()` is the proto zero value, like every
    /// armonik type, so a page size of 0.
    pub fn recommended() -> Self {
        Self {
            sort: Sort::ascending(Default::default()),
            page_size: 100,
            ..Default::default()
        }
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksDetailedResponse")]
pub struct Response {
    pub tasks: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
