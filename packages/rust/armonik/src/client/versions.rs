pub use crate::rpc::versions::Client as Versions;

use crate::client::client_method;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.versions.Versions")]
impl<T: super::Channel> super::ServiceClient<crate::rpc::services::Versions, T> {
    client_method!(ListVersions:
        list()
        -> crate::versions::list::Request => crate::versions::list::Response);
}
