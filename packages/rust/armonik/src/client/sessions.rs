use crate::rpc::services;
use crate::sessions::{
    cancel, close, create, delete, filter, get, list, pause, purge, resume, stop_submission, Raw,
    Sort,
};
use crate::utils::IntoCollection;
use crate::TaskOptions;

/// Service for handling sessions.
pub type Sessions<T = tonic::transport::Channel> = super::ServiceClient<services::Sessions, T>;

impl<T: super::Channel> super::ServiceClient<services::Sessions, T> {
    /// Get a sessions list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = filter::Field>>,
        sort: Sort,
        with_task_options: bool,
        page: i32,
        page_size: i32,
    ) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {
            filters: crate::utils::into_filters(filters),
            sort,
            with_task_options,
            page,
            page_size,
        })
        .await
    }

    /// Get a session by its id.
    pub async fn get(&mut self, session_id: impl Into<String>) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(get::Request {
                session_id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Cancel a session by its id.
    pub async fn cancel(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(cancel::Request {
                session_id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Create a session.
    pub async fn create(
        &mut self,
        partitions: impl IntoIterator<Item = impl Into<String>>,
        default_task_options: TaskOptions,
    ) -> Result<String, super::RequestError> {
        Ok(self
            .call(create::Request {
                default_task_options,
                partition_ids: partitions.into_collect(),
            })
            .await?
            .session_id)
    }

    /// Pause a session by its id.
    pub async fn pause(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(pause::Request {
                session_id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Resume a paused session by its id.
    pub async fn resume(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(resume::Request {
                session_id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Close a session by its id.
    pub async fn close(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(close::Request {
                session_id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Purge a session by its id. Removes Results data.
    pub async fn purge(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(purge::Request {
                session_id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Delete a session by its id. Removes metadata from Results, Sessions and Tasks associated to the session.
    pub async fn delete(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(delete::Request {
                session_id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Stops clients and/or workers from submitting new tasks in the given session.
    pub async fn stop_submission(
        &mut self,
        session_id: impl Into<String>,
        stop_client: bool,
        stop_worker: bool,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(stop_submission::Request {
                session_id: session_id.into(),
                client: stop_client,
                worker: stop_worker,
            })
            .await?
            .session)
    }
}

#[cfg(test)]
#[serial_test::serial(sessions)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Sessions", "ListSessions").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .list(
                crate::sessions::filter::Or {
                    or: vec![crate::sessions::filter::And { and: vec![] }],
                },
                crate::sessions::Sort::default(),
                true,
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "ListSessions").await;
        assert_eq!(after - before, 1);
    }
}
