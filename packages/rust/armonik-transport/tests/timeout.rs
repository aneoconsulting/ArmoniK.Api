//! `GrpcClient__Timeout` and `GrpcClient__RateLimit` reaching the channel.
//!
//! Through a real connection to a real server, measuring what the caller gets: asserting on the parsing
//! alone would say nothing about whether either option reaches the channel.

mod common;

use std::time::{Duration, Instant};

use common::{call, config, serve, SlowService};

#[tokio::test]
async fn a_request_timeout_ends_a_call_the_server_is_too_slow_to_answer() {
    // A server that takes ten seconds against a caller that allows 300ms.
    let endpoint = serve(SlowService::new(Duration::from_secs(10))).await;

    let channel = armonik_transport::connect(config(&endpoint, |args| {
        args.timeout = String::from("300ms");
    }))
    .await
    .expect("connecting should succeed");

    let started = Instant::now();
    let outcome = call(channel).await;
    let elapsed = started.elapsed();

    let status = outcome.expect_err("the call should not have completed");
    assert!(
        elapsed < Duration::from_secs(5),
        "the call took {elapsed:?}, so the timeout was not applied"
    );
    // The message is `tower`'s, reached through `tonic`; asserting on the timing above is the real
    // check, and this only pins that the failure is the timeout rather than something else.
    let rendered = format!("{status:?}").to_lowercase();
    assert!(
        rendered.contains("time") || rendered.contains("elapsed") || rendered.contains("cancel"),
        "unexpected failure: {status:?}"
    );
}

#[tokio::test]
async fn no_timeout_lets_a_slow_call_finish() {
    // The timeout has to bound a call, not shorten one already within its budget. Also the default
    // path: an empty `Timeout` must not impose one.
    let endpoint = serve(SlowService::new(Duration::from_millis(300))).await;

    let channel = armonik_transport::connect(config(&endpoint, |_| {}))
        .await
        .expect("connecting should succeed");

    let answer = call(channel).await.expect("the call should complete");
    assert_eq!(answer.as_ref(), b"late");
}

#[tokio::test]
async fn a_rate_limit_is_accepted_and_still_lets_calls_through() {
    // Testing that `Endpoint::rate_limit` throttles would be testing `tower`. What is worth pinning is
    // that passing the option on does not break the call, which is how wiring one through goes wrong.
    let endpoint = serve(SlowService::new(Duration::ZERO)).await;

    let channel = armonik_transport::connect(config(&endpoint, |args| {
        args.rate_limit = String::from("100/1s");
    }))
    .await
    .expect("connecting should succeed");

    assert_eq!(
        call(channel)
            .await
            .expect("the call should complete")
            .as_ref(),
        b"late"
    );
}

#[test]
fn an_empty_timeout_means_no_timeout_rather_than_a_minute() {
    // What keeps a one-minute deadline off every request of every caller who set nothing.
    let config = config("http://localhost:5001", |_| {});

    assert_eq!(config.timeout, None);
}

#[test]
fn an_empty_connect_timeout_still_means_a_minute() {
    // The mirror image, and the reason the two are treated differently: this 60s *has* always been
    // applied, so it is observable behaviour and changing it would be a regression rather than a fix.
    let config = config("http://localhost:5001", |_| {});

    assert_eq!(config.connect_timeout, Some(Duration::from_secs(60)));
}

#[test]
fn a_timeout_is_parsed_in_the_units_it_was_written_in() {
    for (written, expected) in [
        ("30s", Duration::from_secs(30)),
        ("500ms", Duration::from_millis(500)),
        ("2m", Duration::from_secs(120)),
    ] {
        let config = config("http://localhost:5001", |args| {
            args.timeout = String::from(written);
        });
        assert_eq!(config.timeout, Some(expected), "for {written}");
    }
}
