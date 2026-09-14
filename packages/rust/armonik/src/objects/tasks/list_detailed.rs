use super::{filter, Field, Raw, Sort, SummaryField};

/// Shares its wire form (`ListTasksRequest`) with [`super::list::Request`];
/// a distinct type keeps the two RPCs' requests distinct (request types are
/// injective over RPCs).
#[armonik_macros::message("armonik.api.grpc.v1.tasks.ListTasksRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    /// See [`super::super::list`]'s counterpart: the zero value of a field enum names no field.
    #[test]
    fn recommended_names_a_real_sort_field() {
        let super::Field::Summary(field) = super::Request::recommended().sort.field else {
            panic!("recommended() sorts on a task summary field");
        };
        assert_ne!(i32::from(field), 0);
    }
}

#[armonik_macros::message("armonik.api.grpc.v1.tasks.ListTasksDetailedResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub tasks: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
