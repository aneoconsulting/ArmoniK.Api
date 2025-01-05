#[allow(unused)]
pub(crate) async fn unary_rpc_impl<Response>(
    duration: Option<tokio::time::Duration>,
    failure: Option<tonic::Status>,
    response: impl FnOnce() -> Result<Response, tonic::Status>,
) -> Result<Response, tonic::Status> {
    if let Some(duration) = duration {
        tokio::time::sleep(duration).await;
    }

    if let Some(failure) = failure {
        Err(failure)
    } else {
        response()
    }
}
