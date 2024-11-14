#[allow(unused)]
pub(crate) async fn unary_rpc_impl<Response>(
    duration: Option<tokio::time::Duration>,
    failure: Option<tonic::Status>,
    cancellation_token: tokio_util::sync::CancellationToken,
    response: impl FnOnce() -> Result<Response, tonic::Status>,
) -> Result<Response, tonic::Status> {
    if let Some(duration) = duration {
        cancellation_token
            .run_until_cancelled(tokio::time::sleep(duration))
            .await
            .ok_or(tonic::Status::cancelled("Request has been cancelled"))?;
    } else if cancellation_token.is_cancelled() {
        return Err(tonic::Status::cancelled("Request has been cancelled"));
    }

    if let Some(failure) = failure {
        Err(failure)
    } else {
        response()
    }
}
