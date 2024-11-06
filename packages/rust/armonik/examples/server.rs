use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tracing_subscriber::{prelude::*, EnvFilter};

use armonik::server::SessionsServiceExt;
use armonik::sessions;

pub struct Server;

impl armonik::server::SessionsService for Server {
    /// Get a sessions list using pagination, filters and sorting.
    async fn list(
        self: Arc<Self>,
        _request: sessions::list::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::list::Response, tonic::Status> {
        todo!()
    }

    /// Get a session by its id.
    async fn get(
        self: Arc<Self>,
        _request: sessions::get::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::get::Response, tonic::Status> {
        todo!()
    }

    /// Cancel a session by its id.
    async fn cancel(
        self: Arc<Self>,
        _request: sessions::cancel::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::cancel::Response, tonic::Status> {
        todo!()
    }

    /// Create a session
    async fn create(
        self: Arc<Self>,
        _request: sessions::create::Request,
        cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::create::Response, tonic::Status> {
        tracing::info!("create called");
        if let Some(()) = cancellation_token
            .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_secs(2)))
            .await
        {
            tracing::info!("create returned");
            Ok(sessions::create::Response {
                session_id: String::from("abc"),
            })
        } else {
            tracing::info!("client cancelled RPC");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            tracing::info!("future still running");
            Err(tonic::Status::aborted("client cancelled RPC"))
        }
    }

    /// Pause a session by its id.
    async fn pause(
        self: Arc<Self>,
        _request: sessions::pause::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::pause::Response, tonic::Status> {
        todo!()
    }

    /// Resume a paused session by its id.
    async fn resume(
        self: Arc<Self>,
        _request: sessions::resume::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::resume::Response, tonic::Status> {
        todo!()
    }

    /// Close a session by its id.
    async fn close(
        self: Arc<Self>,
        _request: sessions::close::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::close::Response, tonic::Status> {
        todo!()
    }

    /// Purge a session by its id. Removes Results data.
    async fn purge(
        self: Arc<Self>,
        _request: sessions::purge::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::purge::Response, tonic::Status> {
        todo!()
    }

    /// Delete a session by its id. Removes metadata from Results, Sessions and Tasks associated to the session.
    async fn delete(
        self: Arc<Self>,
        _request: sessions::delete::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::delete::Response, tonic::Status> {
        todo!()
    }

    /// Stops clients and/or workers from submitting new tasks in the given session.
    async fn stop_submission(
        self: Arc<Self>,
        _request: sessions::stop_submission::Request,
        _cancellation_token: CancellationToken,
    ) -> std::result::Result<sessions::stop_submission::Response, tonic::Status> {
        todo!()
    }
}

#[tokio::main]
pub async fn main() -> Result<(), eyre::Report> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    tonic::transport::Server::builder()
        .add_service(Server.sessions_server())
        .serve("127.0.0.1:3456".parse()?)
        .await?;
    Ok(())
}
