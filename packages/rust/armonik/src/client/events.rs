pub use crate::rpc::events::Client as Events;

use crate::client::client_method;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.events.Events")]
impl<T: super::Channel> super::ServiceClient<crate::rpc::services::Events, T> {
    client_method!(GetEvents:
        subscribe(session_id: into<String>, task_filters: filters<crate::tasks::filter::Field>, result_filters: filters<crate::results::filter::Field>, returned_events: iter<crate::events::EventsEnum>)
        -> stream crate::events::subscribe::Request => crate::events::subscribe::Response);
}
