#pragma once
/**
 * @file JsonConfiguration.h
 * @brief Definition of a JSON configuration class that inherits from IConfiguration.
 */
#include "utils/IConfiguration.h"

namespace armonik::api::common::utils {
/**
 * @class JsonConfiguration
 * @brief JSON configuration class that inherits from IConfiguration.
 */
class JsonConfiguration : public IConfiguration {
private:
  /**
   * Parsed values
   */
  std::unordered_map<std::string, std::string> values;

  JsonConfiguration() = default;

public:
  /**
   * @brief Constructor that takes a JSON file path.
   * @param filepath JSON file path to be used for configuration.
   */
  explicit JsonConfiguration(const std::string &filepath);

  static JsonConfiguration fromString(const std::string &json_string);

  /**
   * @brief Get the value associated with the given key.
   * @param string Key to look up.
   * @return The value associated with the key, as a string.
   */
  [[nodiscard]] std::string get(const std::string &string) const override { return values.at(string); }

  /**
   * @brief Set the value associated with the given key.
   * @param string Key to set the value for.
   * @param value Value to set for the key.
   */
  void set(const std::string &string, const std::string &value) override { values.insert_or_assign(string, value); }

  /**
   * @brief Set the values from another IConfiguration object.
   * @param other IConfiguration object to copy values from.
   */
  void set(const IConfiguration &other) override {}
};
} // namespace armonik::api::common::utils
