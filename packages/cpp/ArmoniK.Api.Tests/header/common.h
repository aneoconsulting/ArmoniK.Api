#pragma once

#include "logger/logger.h"
#include "objects.pb.h"
#include <grpcpp/channel.h>
#include <gtest/gtest.h>
#include <memory>

/**
 * @brief Initializes task options creates channel with server address
 *
 * @param channel The gRPC channel to communicate with the server.
 * @param default_task_options The default task options.
 */
void init(std::shared_ptr<grpc::Channel> &channel, armonik::api::grpc::v1::TaskOptions &task_options,
          armonik::api::common::logger::Logger &logger);

/**
 *
 * @param service_name the name of the service providing the rpc methods
 * @param rpc_name the specific rpc to be checked
 * @param endpoint the call endpoint
 * @param num_calls the number of call of rpc
 * @return
 */
bool rpcCalled(const std::string &service_name, const std::string &rpc_name, int num_calls = 1,
               const std::string &endpoint = "http://localhost:4999/calls.json");

/**
 *
 * @param service_name the service name
 * @param endpoint the call endpoint
 * @return
 */
bool all_rpc_called(const std::string &service_name, const std::vector<std::string> &missings = {},
                    const std::string &endpoint = "http://localhost:4999/calls.json");

/**
 *
 * @param endpoint The reset endpoint
 */
void clean_up(const std::string &endpoint = "http://localhost:4999/reset");

/**
 * A fixture class to reset the RPC calls
 */
class MockFixture : public ::testing::Test {
protected:
  static void TearDownTestSuite() { clean_up(); }

  /**
   * Clean up the calls.json file
   */
  void TearDown() override {
    // clean_up();
  }
};
