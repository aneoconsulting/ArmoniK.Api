#include "utils/IConfiguration.h"

#include "options/ComputePlane.h"
#include "utils/JsonConfiguration.h"

namespace armonik::api::common::utils {
IConfiguration &IConfiguration::add_json_configuration(const std::string &file_path) {
  JsonConfiguration::fromPath(*this, file_path);
  return *this;
}

IConfiguration &IConfiguration::add_env_configuration() {
  use_environment_ = true;
  above_env_keys_.clear();
  return *this;
}

options::ComputePlane IConfiguration::get_compute_plane() {
  return *this;
}

void IConfiguration::set(const IConfiguration &other) {
  for(auto&& [key, value] : other.list()){
    set(key, value);
  }
}
void IConfiguration::set(const std::string &key, const std::string &value) {
  if(use_environment_){
    above_env_keys_.insert(key);
  }
  options_[key] = value;
}

std::string IConfiguration::get(const std::string &string) const {
  if(use_environment_ && above_env_keys_.find(string) == above_env_keys_.end()){
    char*  value = std::getenv(string.c_str());
    if(value != nullptr){
      return value;
    }
  }
  auto position =  options_.find(string);
  return position == options_.end() ? "" : position->second;
}

const std::map<std::string, std::string> &IConfiguration::list() const {
  return options_;
}

} // namespace armonik::api::common::utils
