#include "utils/IConfiguration.h"

#include "options/ComputePlane.h"
#include "utils/EnvConfiguration.h"
#include "utils/JsonConfiguration.h"

namespace armonik::api::common::utils {
IConfiguration &IConfiguration::add_json_configuration(const std::string &file_path) {
  JsonConfiguration json_configuration(file_path);

  return *this;
}

IConfiguration &IConfiguration::add_env_configuration() {
  EnvConfiguration env_config;

  return *this;
}

options::ComputePlane IConfiguration::get_compute_plane() { return *this; }

} // namespace armonik::api::common::utils
