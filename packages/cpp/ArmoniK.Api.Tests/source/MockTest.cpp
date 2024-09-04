#include <curl/curl.h>
#include <grpcpp/create_channel.h>
#include <gtest/gtest.h>
#include <json/json.h>

#include "common.h"
#include "exceptions/ArmoniKApiException.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "channel/ChannelFactory.h"
#include "sessions/SessionsClient.h"

using Logger = armonik::api::common::logger::Logger;

size_t WriteCallback(void *ptr, size_t size, size_t num_elt, std::string *data) {
  data->append((char *)ptr, size * num_elt);
  return size * num_elt;
}

bool rpcCalled(const std::string &service_name, const std::string &rpc_name, int num_calls,
               const std::string &endpoint) {

  auto curl = curl_easy_init();
  std::string read_buffer;
  std::cout << endpoint << std::endl;
  if (curl) {
    curl_easy_setopt(curl, CURLOPT_URL, endpoint.c_str());
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, WriteCallback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &read_buffer);

    auto res = curl_easy_perform(curl);

    if (res != CURLE_OK) {
      std::cout << "Request failed: " << curl_easy_strerror(res) << std::endl;
    }
    curl_easy_cleanup(curl);
  }

  Json::CharReaderBuilder readerBuilder;
  Json::Value response_json;
  std::istringstream jsonStream(read_buffer);
  std::string errs;

  bool res = Json::parseFromStream(readerBuilder, jsonStream, &response_json, &errs);
  if (!res) {
    std::cerr << "Failed to parse JSON: " << errs << std::endl;
  }
  if (response_json[service_name][rpc_name] == num_calls) {
    return true;
  }
  std::cout << "Given num calls: " << num_calls << std::endl;
  std::cout << "Actual num calls: " << response_json[service_name][rpc_name] << std::endl;
  return false;
}

bool all_rpc_called(const std::string &service_name, const std::vector<std::string> &missings,
                    const std::string &endpoint) {
  auto curl = curl_easy_init();
  std::string read_buffer;
  std::cout << endpoint << std::endl;
  if (curl) {
    curl_easy_setopt(curl, CURLOPT_URL, endpoint.c_str());
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, WriteCallback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &read_buffer);

    auto res = curl_easy_perform(curl);

    if (res != CURLE_OK) {
      std::cout << "Request failed: " << curl_easy_strerror(res) << std::endl;
    }
    curl_easy_cleanup(curl);
  }

  Json::CharReaderBuilder readerBuilder;
  Json::Value response_json;
  std::istringstream jsonStream(read_buffer);
  std::string errs;

  bool res = Json::parseFromStream(readerBuilder, jsonStream, &response_json, &errs);
  if (!res) {
    std::cerr << "Failed to parse JSON: " << errs << std::endl;
  }

  std::vector<std::string> missing_rpcs;
  for (auto &rpc_name : response_json[service_name].getMemberNames()) {
    if (response_json[service_name][rpc_name] == 0) {
      missing_rpcs.push_back(rpc_name);
    }
  }
  if (!missing_rpcs.empty()) {
    if (missing_rpcs == missings) {
      return true;
    }
    std::cout << "RPCs not implemented in " << service_name << " service: \n";
    for (const auto &str : missing_rpcs) {
      std::cout << str << '\n';
    }
    return false;
  }
  return true;
}

void clean_up(const std::string &endpoint) {
  auto curl = curl_easy_init();
  std::string read_buffer;
  if (curl) {
    curl_easy_setopt(curl, CURLOPT_URL, endpoint.c_str());
    curl_easy_setopt(curl, CURLOPT_POST, 1L);
    auto res = curl_easy_perform(curl);
    if (res != CURLE_OK) {
      std::cout << "Request failed: " << curl_easy_strerror(res) << std::endl;
    }
    curl_easy_cleanup(curl);
  }
}

using Logger = armonik::api::common::logger::Logger;

TEST_F(MockFixture, connect) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  armonik::api::common::utils::Configuration configuration;
  // auto server = std::make_shared<EnvConfiguration>(configuration_t);

  configuration.add_json_configuration("appsettings.json").add_env_configuration();

  std::string server_address = configuration.get("Grpc__EndPoint");

  armonik::api::client::ChannelFactory channel_factory(configuration, log);

  channel = channel_factory.create_channel();

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string response;
  ASSERT_NO_THROW(response = client.create_session(task_options));
  ASSERT_FALSE(response.empty());
  ASSERT_TRUE(rpcCalled("Sessions", "CreateSession"));
}