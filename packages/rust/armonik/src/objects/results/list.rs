use super::{filter, Raw, Sort};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.ListResultsRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub page: i32,
    pub page_size: i32,
}

impl Request {
    /// A first page of 100 results, sorted ascending on the default field, with
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
#[armonik(message = "armonik.api.grpc.v1.results.ListResultsResponse")]
pub struct Response {
    pub results: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
