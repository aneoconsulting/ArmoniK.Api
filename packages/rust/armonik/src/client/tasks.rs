use crate::rpc::services;

/// Service for handling tasks.
pub type Tasks<T = tonic::transport::Channel> = super::ServiceClient<services::Tasks, T>;

#[cfg(test)]
#[serial_test::serial(tasks)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Tasks", "ListTasks").await;
        let mut client = Client::new().await.unwrap().into_tasks();
        client
            .list(
                crate::tasks::filter::Or {
                    or: vec![crate::tasks::filter::And { and: vec![] }],
                },
                crate::tasks::Sort::default(),
                true,
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "ListTasks").await;
        assert_eq!(after - before, 1);
    }
}
