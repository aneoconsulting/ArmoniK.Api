#pragma once
#include <future>
#include <string>

#include "Worker/ITaskHandler.h"
#include "Worker/TaskHandler.h"

#include "agent_common.pb.h"
#include "agent_service.grpc.pb.h"

//#include "SessionContext.h"


/**
 * @brief The TaskHandler classprovides methods to create and handle tasks
 * 
 */
class TaskHandler : public ITaskHandler
{
private:

    grpc::ClientContext context_;
    std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> stub_;

public:

    static std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>> task_chunk_stream(
        const armonik::api::grpc::v1::TaskRequest& task_request,
        bool is_last, size_t chunk_max_size);


    static auto to_request_stream(
        const std::vector<armonik::api::grpc::v1::TaskRequest>& task_requests,
        const armonik::api::grpc::v1::TaskOptions& task_options,
        size_t chunk_max_size)->std::vector<std::future<std::vector<armonik::api::grpc::v1::agent::
        CreateTaskRequest>>>;

    /**
     * @brief Create a tasks async object
     * 
     * @param channel 
     * @param session_id 
     * @param task_options 
     * @param task_requests 
     * @return std::future<armonik::api::grpc::v1::agent::CreateTaskReply> 
     */
    static std::future<armonik::api::grpc::v1::agent::CreateTaskReply> create_tasks_async(
        const std::shared_ptr<grpc::ChannelInterface>& channel,
        std::string& session_id, const armonik::api::grpc::v1::TaskOptions& task_options,
        const std::vector<armonik::api::grpc::v1::TaskRequest>& task_requests);

};