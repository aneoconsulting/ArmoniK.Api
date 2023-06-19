#include "submitter/SessionContext.h"

SessionContext::SessionContext(armonik::api::grpc::v1::TaskOptions task_options) : task_options_(
    std::move(task_options))
{
}

void SessionContext::set_session_id(const std::string& session_id)
{
  session_id_ = session_id;
}
