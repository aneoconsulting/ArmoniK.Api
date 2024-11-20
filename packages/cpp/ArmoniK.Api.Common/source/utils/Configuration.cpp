#include "utils/Configuration.h"

#include "options/ComputePlane.h"
#include "options/ControlPlane.h"
#include "utils/JsonConfiguration.h"

constexpr char armonik::api::common::options::ControlPlane::CaCertKey[];
constexpr char armonik::api::common::options::ControlPlane::EndpointKey[];
constexpr char armonik::api::common::options::ControlPlane::AllowUnsafeConnectionKey[];
constexpr char armonik::api::common::options::ControlPlane::UserCertKey[];
constexpr char armonik::api::common::options::ControlPlane::UserKeyKey[];
constexpr char armonik::api::common::options::ControlPlane::UserP12Key[];
constexpr char armonik::api::common::options::ControlPlane::KeepAliveTimeKey[];
constexpr char armonik::api::common::options::ControlPlane::KeepAliveTimeIntervalKey[];
constexpr char armonik::api::common::options::ControlPlane::MaxIdleTimeKey[];
constexpr char armonik::api::common::options::ControlPlane::MaxAttemptsKey[];
constexpr char armonik::api::common::options::ControlPlane::BackoffMultiplierKey[];
constexpr char armonik::api::common::options::ControlPlane::InitialBackOffKey[];
constexpr char armonik::api::common::options::ControlPlane::MaxBackOffKey[];
constexpr char armonik::api::common::options::ControlPlane::RequestTimeoutKey[];

namespace armonik {
namespace api {
namespace common {
namespace utils {

Configuration &Configuration::add_json_configuration(absl::string_view file_path) {
  JsonConfiguration::fromPath(*this, file_path);
  return *this;
}

Configuration &Configuration::add_env_configuration() {
  use_environment_ = true;
  above_env_keys_.clear();
  return *this;
}

options::ComputePlane Configuration::get_compute_plane() const { return *this; }

void Configuration::set(const Configuration &other) {
  for (auto &&kv : other.list()) {
    set(kv.first, kv.second);
  }
}
void Configuration::set(const std::string &key, const std::string &value) {
  if (use_environment_) {
    above_env_keys_.insert(key);
  }
  options_[key] = value;
}

std::string Configuration::get(const std::string &string) const {
  if (use_environment_ && above_env_keys_.find(string) == above_env_keys_.end()) {
    char *value = std::getenv(string.c_str());
    if (value != nullptr) {
      return value;
    }
  }
  auto position = options_.find(string);
  return position == options_.end() ? "" : position->second;
}

const std::map<std::string, std::string> &Configuration::list() const { return options_; }
options::ControlPlane Configuration::get_control_plane() const { return *this; }

} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
