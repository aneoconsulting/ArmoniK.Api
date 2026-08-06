use armonik::events;
use armonik::reexports::tokio_stream::StreamExt;
use armonik::server::{EventsServiceExt, RequestContext};

#[macro_use]
mod common;

rpc_tests! {
    client: into_events;
    server: EventsService, events_server;
    mock: "Events";
    fake { dropped: tokio_util::sync::CancellationToken }

    rpc server_stream subscribe {
        request: events::subscribe::Request {
            session_id: String::from("rpc-subscribe-input"),
            task_filters: armonik::tasks::filter::Or::default(),
            result_filters: armonik::results::filter::Or::default(),
            returned_events: vec![events::EventsEnum::UNSPECIFIED],
        },
        convenience: subscribe(
            "rpc-subscribe-input",
            armonik::tasks::filter::Or::default(),
            armonik::results::filter::Or::default(),
            [events::EventsEnum::UNSPECIFIED],
        ),
        check: |mut stream| async move {
            let event = stream.next().await.unwrap().unwrap();

            assert_eq!(event.session_id, "rpc-subscribe-input");
            match event.update {
                events::Update::NewResult(new_result) => {
                    assert_eq!(new_result.result_id, "rpc-subscribe-output")
                }
                event => panic!("expected a NewResult, but got {event:?}"),
            }
        },
    }

    manual {
        // Endless on purpose: the drop test below needs a stream that only ends
        // when the response is dropped.
        async fn subscribe(
            self: std::sync::Arc<Self>,
            request: events::subscribe::Request,
            _context: RequestContext,
        ) -> Result<
            impl armonik::reexports::tokio_stream::Stream<
                    Item = Result<events::subscribe::Response, tonic::Status>,
                > + Send,
            tonic::Status,
        > {
            let end_ct = self.dropped.clone();
            Ok(async_stream::try_stream! {
                let _drop_guard = end_ct.drop_guard();
                loop {
                    if let Some(duration) = self.wait {
                        tokio::time::sleep(duration).await;
                    }

                    if let Some(failure) = self.failure.clone() {
                        Err(failure)?
                    }

                    yield events::subscribe::Response {
                        session_id: request.session_id.clone(),
                        update: events::Update::NewResult(events::NewResult {
                            result_id: String::from("rpc-subscribe-output"),
                            ..Default::default()
                        }),
                    };
                }
            })
        }
    }
}

/// The server-side stream is endless, so dropping the response has to be what
/// tears the handler down.
#[tokio::test]
async fn subscribe_drop_cancels_the_handler() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = armonik::Client::with_channel(
        Service {
            dropped: cancellation_token.clone(),
            ..Default::default()
        }
        .events_server(),
    )
    .into_events();

    let mut response = client
        .subscribe(
            "rpc-subscribe-input",
            armonik::tasks::filter::Or::default(),
            armonik::results::filter::Or::default(),
            [events::EventsEnum::UNSPECIFIED],
        )
        .await
        .unwrap();

    _ = response.next().await.unwrap().unwrap();

    std::mem::drop(response);

    if cancellation_token
        .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_millis(100)))
        .await
        .is_some()
    {
        panic!("Expected a cancel, but got a timeout");
    }
}
