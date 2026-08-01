//! Opening many connections in a short window.
//!
//! This mirrors ArmoniK's `MultipleChannels` client test, which builds up to a hundred
//! channels at once. On Windows that pattern is what exhausts the ephemeral port range, and
//! `GrpcClient__ReusePorts` exists to defer port allocation so it does not.

use std::time::Duration;

use armonik_transport::ClientConfig;

mod common;

use common::SlowService;

async fn spawn_server() -> String {
    common::serve(SlowService::new(Duration::ZERO)).await
}

fn config(endpoint: &str, reuse_ports: bool) -> ClientConfig {
    common::config(endpoint, |args| args.reuse_ports = Some(reuse_ports))
}

/// Build `count` independent channels at once and call through every one of them.
///
/// Each channel is its own TCP connection, which is the point: a shared channel would multiplex
/// over one socket and never touch the port range.
async fn open_channels(config: ClientConfig, count: usize) -> Result<(), String> {
    let mut channels = Vec::with_capacity(count);
    for index in 0..count {
        let channel = armonik_transport::connect(config.clone())
            .await
            .map_err(|error| format!("connection {index} failed: {error}"))?;
        channels.push(channel);
    }

    for (index, channel) in channels.into_iter().enumerate() {
        let response = common::call(channel)
            .await
            .map_err(|error| format!("call {index} failed: {error}"))?;
        if response != common::REPLY {
            return Err(format!("call {index} returned {response:?}"));
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn many_channels_without_port_reuse() {
    let server = spawn_server().await;

    open_channels(config(&server, false), 100)
        .await
        .expect("100 channels should all connect and answer");
}

#[tokio::test(flavor = "multi_thread")]
async fn many_channels_with_port_reuse() {
    let server = spawn_server().await;

    // Same load, with port reuse on. On Windows this takes the connector that sets
    // `SO_REUSE_UNICASTPORT`; elsewhere the option is accepted and does nothing.
    open_channels(config(&server, true), 100)
        .await
        .expect("100 channels should all connect and answer with port reuse on");
}

#[tokio::test(flavor = "multi_thread")]
async fn port_reuse_does_not_change_what_the_call_returns() {
    let server = spawn_server().await;

    for reuse_ports in [false, true] {
        let channel = armonik_transport::connect(config(&server, reuse_ports))
            .await
            .expect("connect");

        let response = common::call(channel).await.expect("call");
        assert_eq!(
            response,
            common::REPLY,
            "reuse_ports={reuse_ports} changed the response"
        );
    }
}
