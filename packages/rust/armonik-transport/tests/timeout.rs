//! `GrpcClient__Timeout` and `GrpcClient__RateLimit`, now that they reach the channel.
//!
//! Both were parsed into `ClientConfig` and then dropped on the floor. A test that only asserted the
//! parsing would have passed before this change as well, so these go through a real connection to a
//! real server and measure what the caller actually gets.

mod common;

use std::time::{Duration, Instant};

use common::{call, config, serve, SlowService};

#[tokio::test]
async fn a_request_timeout_ends_a_call_the_server_is_too_slow_to_answer() {
    // The server takes ten seconds; the caller allows 300ms. Before this change the option was parsed
    // and ignored, so the call waited for the full ten.
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
    // The other half: the timeout has to bound a call, not shorten one that was within its budget. Also
    // the default path — an empty `Timeout` must not impose one.
    let endpoint = serve(SlowService::new(Duration::from_millis(300))).await;

    let channel = armonik_transport::connect(config(&endpoint, |_| {}))
        .await
        .expect("connecting should succeed");

    let answer = call(channel).await.expect("the call should complete");
    assert_eq!(answer.as_ref(), b"late");
}

#[tokio::test]
async fn a_rate_limit_is_accepted_and_still_lets_calls_through() {
    // `Endpoint::rate_limit` is `tower`'s, and testing that it throttles would be testing `tower`. What
    // is worth pinning here is that passing it on does not break a call — the failure mode of wiring an
    // option through incorrectly.
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
    // The field is documented as defaulting to no timeout, and nothing applied it until now, so the
    // `Some(60s)` it used to parse to was never observable. This is the assertion that keeps a
    // one-minute deadline from appearing on every request of every caller.
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
