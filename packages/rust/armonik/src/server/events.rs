use std::sync::Arc;

use crate::api::v3;
use crate::events;

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
    fn events_server(self) -> v3::events::events_server::EventsServer<Self>
    where
        Self: Sized;
}

impl<T: EventsService + Send + Sync + 'static> EventsServiceExt for T {
    fn events_server(self) -> v3::events::events_server::EventsServer<Self> {
        v3::events::events_server::EventsServer::new(self)
    }
}

#[crate::reexports::async_trait]
impl<T: EventsService + Send + Sync + 'static> v3::events::events_server::Events for T {
    type GetEventsStream = crate::server::ServerStream<v3::events::EventSubscriptionResponse>;
    async fn get_events(
        self: Arc<Self>,
        request: tonic::Request<v3::events::EventSubscriptionRequest>,
    ) -> Result<tonic::Response<Self::GetEventsStream>, tonic::Status> {
        super::impl_trait_methods!(stream server (self, request) {EventsService::subscribe})
    }
}
