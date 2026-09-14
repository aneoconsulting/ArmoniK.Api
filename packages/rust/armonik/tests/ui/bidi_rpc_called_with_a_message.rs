//! A bidirectional RPC takes a `Stream` of its request messages, like a client-streaming one.

use armonik::reexports::tonic;

async fn misuse(channel: tonic::transport::Channel) {
    let mut client = armonik::Client::with_channel(channel).into_results();

    // Not awaited: the bound is on `call`'s parameter, so one call reports it once. Awaiting the
    // future repeats the same error twice more, which the snapshot does not need.
    let _call = client.call(armonik::results::watch::Request::default());
}

fn main() {}
