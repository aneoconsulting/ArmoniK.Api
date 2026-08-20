//! A client-streaming RPC takes a `Stream` of its request messages. One message is not an input
//! for it, however much it looks like one.

use armonik::reexports::tonic;

async fn misuse(channel: tonic::transport::Channel) {
    let mut client = armonik::Client::with_channel(channel).into_results();

    // Not awaited: the bound is on `call`'s parameter, so one call reports it once. Awaiting the
    // future repeats the same error twice more, which the snapshot does not need.
    let _call = client.call(armonik::results::upload::Request::DataChunk(
        armonik::reexports::bytes::Bytes::new(),
    ));
}

fn main() {}
