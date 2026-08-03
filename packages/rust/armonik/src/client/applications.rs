use crate::applications::{filter, list, Sort};
use crate::rpc::services;

/// Service for handling applications.
pub type Applications<T = tonic::transport::Channel> = super::ServiceClient<services::Applications, T>;

impl<T: super::Channel> super::ServiceClient<services::Applications, T> {
    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = filter::Field>>,
        sort: Sort,
        page: i32,
        page_size: i32,
    ) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {
            filters: crate::utils::into_filters(filters),
            sort,
            page,
            page_size,
        })
        .await
    }
}

#[cfg(test)]
#[serial_test::serial(applications)]
mod tests {
    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Applications", "ListApplications").await;
        let mut client = Client::new().await.unwrap().into_applications();
        client
            .list(
                crate::applications::filter::Or {
                    or: vec![crate::applications::filter::And { and: vec![] }],
                },
                crate::applications::Sort::default(),
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Applications", "ListApplications").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn list_call() {
        let before = Client::get_nb_request("Applications", "ListApplications").await;
        let mut client = Client::new().await.unwrap().into_applications();
        client
            .call(crate::applications::list::Request {
                page_size: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Applications", "ListApplications").await;
        assert_eq!(after - before, 1);
    }
}
