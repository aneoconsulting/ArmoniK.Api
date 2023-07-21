
#include "utils/JsonConfiguration.h"
#include <iostream>
#include <simdjson.h>

using namespace simdjson;

/**
 * @brief Populates the given config with the given json element using a prefix
 * @param config JsonConfiguration to populate
 * @param prefix Prefix for the key
 * @param element json element
 */
void populate(API_COMMON_NAMESPACE::utils::IConfiguration &config, const std::string &prefix,
              const dom::element &element) {
  switch (element.type()) {
  case dom::element_type::ARRAY: {
    std::string previous_prefix = prefix.empty() ? "" : prefix + "__";
    int i = 0;
    for (auto &&child : dom::array(element)) {
      populate(config, previous_prefix + std::to_string(i++), child);
    }
  } break;
  case dom::element_type::OBJECT: {
    std::string previous_prefix = prefix.empty() ? "" : prefix + "__";
    for (dom::key_value_pair field : dom::object(element)) {
      populate(config, previous_prefix + std::string(field.key), field.value);
    }
  } break;
  default:
    config.set(prefix, std::string{element.get_string().take_value()});
    break;
  }
}

API_COMMON_NAMESPACE::utils::JsonConfiguration::JsonConfiguration(const std::string &json_path) {
  fromPath(*this, json_path);
}

API_COMMON_NAMESPACE::utils::JsonConfiguration
API_COMMON_NAMESPACE::utils::JsonConfiguration::fromString(const std::string &json_string) {
  JsonConfiguration config;
  fromString(config, json_string);
  return config;
}
void API_COMMON_NAMESPACE::utils::JsonConfiguration::fromPath(API_COMMON_NAMESPACE::utils::IConfiguration &config,
                                                              std::string_view filepath) {
  dom::parser parser;
  dom::element elem;
  try {
    elem = parser.load(std::string(filepath));
  } catch (const std::exception &e) {
    std::cerr << "Unable to load json file " << filepath << " : " << e.what();
  }
  populate(config, "", elem);
}
void API_COMMON_NAMESPACE::utils::JsonConfiguration::fromString(API_COMMON_NAMESPACE::utils::IConfiguration &config,
                                                                const std::string &json_string) {
  dom::parser parser;
  populate(config, "", parser.parse(padded_string(json_string)));
}
