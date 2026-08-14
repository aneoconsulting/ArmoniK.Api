use super::{filter, Field, Sort, Summary, SummaryField};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub with_errors: bool,
    pub page: i32,
    pub page_size: i32,
}

impl Request {
    /// A first page of 100 tasks, sorted ascending on the task id, with no
    /// filter. `Default::default()` is the proto zero value, like every armonik
    /// type, so a page size of 0 and a sort field naming no field at all: this
    /// names one.
    pub fn recommended() -> Self {
        Self {
            sort: Sort::ascending(Field::Summary(SummaryField::TaskId)),
            page_size: 100,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    /// The sort field a request carries is a proto enum, whose zero value names no field. A request
    /// sorted on it asks the control plane to sort on `UNSPECIFIED`, which is why `recommended()`
    /// cannot reach it through `Default`.
    #[test]
    fn recommended_names_a_real_sort_field() {
        let super::Field::Summary(field) = super::Request::recommended().sort.field else {
            panic!("recommended() sorts on a task summary field");
        };
        assert_ne!(i32::from(field), 0);
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksResponse")]
pub struct Response {
    pub tasks: Vec<Summary>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
