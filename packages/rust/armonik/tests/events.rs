use std::sync::Arc;

use armonik::{events, reexports::tokio_stream::StreamExt, server::EventsServiceExt};

mod common;

struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
    dropped: tokio_util::sync::CancellationToken,
}

impl armonik::server::EventsService for Service {
    async fn subscribe(
        self: Arc<Self>,
        request: events::subscribe::Request,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Result<
        impl tonic::codegen::tokio_stream::Stream<
                Item = Result<events::subscribe::Response, tonic::Status>,
            > + Send,
        tonic::Status,
    > {
        let end_ct = self.dropped.clone();
        Ok(async_stream::try_stream! {
            let _drop_guard = end_ct.drop_guard();
            loop {
                if let Some(duration) = self.wait.clone() {
                    cancellation_token
                        .run_until_cancelled(tokio::time::sleep(duration))
                        .await
                        .ok_or(tonic::Status::cancelled("Request has been cancelled"))?;
                } else if cancellation_token.is_cancelled() {
                    Err(tonic::Status::cancelled("Request has been cancelled"))?;
                }

                if let Some(failure) = self.failure.clone() {
                    Err(failure)?
                }

                yield events::subscribe::Response{
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

#[tokio::test]
async fn subscribe() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = armonik::Client::with_channel(
        Service {
            failure: None,
            wait: None,
            dropped: cancellation_token.clone(),
        }
        .events_server(),
    )
    .events();

    let mut response = client
        .subscribe(
            "rpc-subscribe-input",
            armonik::tasks::filter::Or::default(),
            armonik::results::filter::Or::default(),
            [events::EventsEnum::Unspecified],
        )
        .await
        .unwrap();

    let event = response.next().await.unwrap().unwrap();

    assert_eq!(event.session_id, "rpc-subscribe-input");
    match event.update {
        events::Update::NewResult(new_result) => {
            assert_eq!(new_result.result_id, "rpc-subscribe-output")
        }
        event => panic!("expected a NewResult, but got {event:?}"),
    }

    match response.next().await {
        Some(Ok(event)) => eprintln!("Got event: {event:?}"),
        Some(Err(err)) => eprintln!("Got error: {err:?}"),
        None => {
            eprintln!("Got end of stream");
        }
    }

    std::mem::drop(response);

    if cancellation_token
        .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_millis(10)))
        .await
        .is_some()
    {
        panic!("Expected a cancel, but got a timeout");
    }
}
