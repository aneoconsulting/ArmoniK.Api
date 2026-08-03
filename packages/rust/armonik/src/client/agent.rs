use futures::Stream;
use snafu::ResultExt;

use crate::agent::create_tasks;
use crate::rpc::services;

/// The Agent gRPC service, exposed to workers for spawning subtasks and
/// exchanging data.
pub type Agent<T = tonic::transport::Channel> = super::ServiceClient<services::Agent, T>;

impl<T: super::Channel> super::ServiceClient<services::Agent, T> {
    pub async fn create_tasks(
        &mut self,
        request: impl Stream<Item = create_tasks::Request> + Send + 'static,
    ) -> Result<Vec<create_tasks::Status>, super::RequestError> {
        let response = self.call_streaming(request).await?;

        match response {
            create_tasks::Response::Status {
                communication_token: _,
                statuses,
            } => Ok(statuses),
            create_tasks::Response::Error {
                communication_token: _,
                error,
            } => Err(tonic::Status::internal(error)).context(super::GrpcSnafu {}),
        }
    }
}

#[cfg(test)]
#[serial_test::serial(agent)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn submit() {
        let before = Client::get_nb_request("Agent", "SubmitTasks").await;
        let mut client = Client::new().await.unwrap().into_agent();
        client
            .submit_tasks(
                "token",
                "session-id",
                None,
                Vec::<crate::agent::submit_tasks::RequestItem>::new(),
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "SubmitTasks").await;
        assert_eq!(after - before, 1);
    }
}
