//! A unary RPC takes its request message. A `Stream` of them is not an input for it.

use armonik::reexports::tonic;

async fn misuse(channel: tonic::transport::Channel) {
    let mut client = armonik::Client::with_channel(channel).into_versions();

    // Not awaited: the bound is on `call`'s parameter, so one call reports it once. Awaiting the
    // future repeats the same error twice more, which the snapshot does not need.
    let _call = client.call(futures::stream::iter([armonik::versions::list::Request {}]));
}

fn main() {}
