#include <curl/curl.h>
#include <grpcpp/create_channel.h>
#include <gtest/gtest.h>
#include <simdjson.h>

#include "common.h"
#include "exceptions/ArmoniKApiException.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "channel/ChannelFactory.h"
#include "sessions/SessionsClient.h"

using Logger = armonik::api::common::logger::Logger;
using namespace simdjson;

size_t WriteCallback(void *ptr, size_t size, size_t num_elt, std::string *data) {
  data->append((char *)ptr, size * num_elt);
  return size * num_elt;
}

bool rpcCalled(absl::string_view service_name, absl::string_view rpc_name, int num_calls, absl::string_view endpoint) {

  armonik::api::common::utils::Configuration config;
  config.add_json_configuration("appsettings.json").add_env_configuration();
  std::string call_endpoint = config.get("Http__EndPoint") + "/calls.json";
  auto curl = curl_easy_init();
  std::string read_buffer;
  if (curl) {
    curl_easy_setopt(curl, CURLOPT_URL, call_endpoint.c_str());
    curl_easy_setopt(curl, CURLOPT_VERBOSE, 1L);
    curl_easy_setopt(curl, CURLOPT_CAINFO, config.get("Grpc__CaCert").c_str());
    if (config.get("Grpc__mTLS") == "true") {
      curl_easy_setopt(curl, CURLOPT_SSLCERT, config.get("Grpc__ClientCert").c_str());
      curl_easy_setopt(curl, CURLOPT_SSLKEY, config.get("Grpc__ClientKey").c_str());
    }
    curl_easy_setopt(curl, CURLOPT_HTTP_VERSION, CURL_HTTP_VERSION_1_1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, WriteCallback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &read_buffer);

    auto res = curl_easy_perform(curl);

    if (res != CURLE_OK) {
      std::cout << "Request failed: " << curl_easy_strerror(res) << std::endl;
    }
    curl_easy_cleanup(curl);
  }

  dom::parser parser;

  try {
    dom::element response_json = parser.parse(read_buffer);
    if (response_json[service_name.data()][rpc_name.data()].get_int64() == num_calls) {
      return true;
    }
    std::cout << "Given number of RPC calls " << num_calls << std::endl;
    std::cout << "Actual number of RPC calls " << response_json[service_name][rpc_name] << std::endl;
  } catch (const simdjson_error &e) {
    std::cerr << "Failed to parse JSON: " << e.what() << std::endl;
  }
  return false;
}

bool all_rpc_called(absl::string_view service_name, const std::vector<std::string> &missings,
                    absl::string_view endpoint) {
  armonik::api::common::utils::Configuration config;
  config.add_json_configuration("appsettings.json").add_env_configuration();
  std::string call_endpoint = config.get("Http__EndPoint") + "/calls.json";
  auto curl = curl_easy_init();
  std::string read_buffer;
  if (curl) {
    curl_easy_setopt(curl, CURLOPT_URL, call_endpoint.c_str());
    curl_easy_setopt(curl, CURLOPT_CAINFO, config.get("Grpc__CaCert").c_str());
    if (config.get("Grpc__mTLS") == "true") {
      curl_easy_setopt(curl, CURLOPT_SSLCERT, config.get("Grpc__ClientCert").c_str());
      curl_easy_setopt(curl, CURLOPT_SSLKEY, config.get("Grpc__ClientKey").c_str());
    }
    curl_easy_setopt(curl, CURLOPT_HTTP_VERSION, CURL_HTTP_VERSION_1_1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, WriteCallback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &read_buffer);

    auto res = curl_easy_perform(curl);

    if (res != CURLE_OK) {
      std::cout << "Request failed: " << curl_easy_strerror(res) << std::endl;
    }
    curl_easy_cleanup(curl);
  }

  dom::parser parser;

  try {
    dom::element response_json = parser.parse(read_buffer);

    std::vector<std::string> missing_rpcs;
    for (auto rpc_name : response_json[service_name.data()].get_array()) {
      if (response_json[service_name.data()][rpc_name].get_int64() == 0) {
        missing_rpcs.emplace_back(rpc_name.get_string().value().data());
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

  } catch (const simdjson_error &e) {
    std::cerr << "Failed to parse JSON: " << e.what() << std::endl;
  }
  return true;
}

void clean_up() {
  armonik::api::common::utils::Configuration config;
  config.add_json_configuration("appsettings.json").add_env_configuration();
  std::string reset_endpoint = config.get("Http__EndPoint") + "/reset";
  auto curl = curl_easy_init();
  std::string read_buffer;
  if (curl) {
    curl_easy_setopt(curl, CURLOPT_URL, reset_endpoint.c_str());
    curl_easy_setopt(curl, CURLOPT_CAINFO, config.get("Grpc__CaCert").c_str());
    if (config.get("Grpc__mTLS") == "true") {
      curl_easy_setopt(curl, CURLOPT_SSLCERT, config.get("Grpc__ClientCert").c_str());
      curl_easy_setopt(curl, CURLOPT_SSLKEY, config.get("Grpc__ClientKey").c_str());
    }
    curl_easy_setopt(curl, CURLOPT_POST, 1L);
    curl_easy_setopt(curl, CURLOPT_HTTP_VERSION, CURL_HTTP_VERSION_1_1);
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
