pub use crate::rpc::applications::Client as Applications;

use crate::client::client_method;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.applications.Applications")]
impl<T: super::Channel> super::ServiceClient<crate::rpc::services::Applications, T> {
    client_method!(ListApplications:
        list(filters: filters<crate::applications::filter::Field>, sort: plain<crate::applications::Sort>, page: plain<i32>, page_size: plain<i32>)
        -> crate::applications::list::Request => crate::applications::list::Response);
}
