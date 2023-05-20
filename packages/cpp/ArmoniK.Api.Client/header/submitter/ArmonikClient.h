#pragma once
#include <iostream>
#include <memory>
#include <string>

#include <grpcpp/grpcpp.h>
#include "submitter_service.pb.h"
#include "submitter_common.pb.h"
#include "submitter_service.grpc.pb.h"


class SubmitterClient {
private:
  grpc::ClientContext context_;
  std::unique_ptr<armonik::api::grpc::v1::submitter::Submitter::Stub> stub_;

 public:
  SubmitterClient(const std::shared_ptr<grpc::Channel>& channel)
      : stub_(armonik::api::grpc::v1::submitter::Submitter::NewStub(channel)) {}

  

  std::string CreateSession(armonik::api::grpc::v1::TaskOptions task_options,
                            const std::vector<std::string>& partition_ids);

  void CancelSession(const std::string& session_id);


  void CreateLargeTask(const std::string& session_id,
                       const armonik::api::grpc::v1::TaskOptions& options,
                       const std::vector<std::string>& expected_output_keys,
                       const std::vector<std::string>& data_dependencies,
                       const std::vector<uint8_t>& payload);

};

