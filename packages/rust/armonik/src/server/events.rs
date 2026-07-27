use std::sync::Arc;

use crate::events;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::events::events_server as stub;

pub trait EventsService {
    fn subscribe(
        self: Arc<Self>,
        request: events::subscribe::Request,
        context: crate::server::RequestContext,
    ) -> impl std::future::Future<
        Output = Result<
            impl tonic::codegen::tokio_stream::Stream<
                    Item = Result<events::subscribe::Response, tonic::Status>,
                > + Send,
            tonic::Status,
        >,
    > + Send;
}

pub trait EventsServiceExt {
    fn events_server(self) -> stub::EventsServer<Self>
    where
        Self: Sized;
}

impl<T: EventsService + Send + Sync + 'static> EventsServiceExt for T {
    fn events_server(self) -> stub::EventsServer<Self> {
        stub::EventsServer::new(self)
    }
}

#[crate::reexports::async_trait]
impl<T: EventsService + Send + Sync + 'static> stub::Events for T {
    type GetEventsStream = crate::server::ServerStream<crate::events::subscribe::Response>;
    async fn get_events(
        self: Arc<Self>,
        request: tonic::Request<crate::events::subscribe::Request>,
    ) -> Result<tonic::Response<Self::GetEventsStream>, tonic::Status> {
        super::impl_trait_methods!(stream server (self, request) {EventsService::subscribe})
    }
}
