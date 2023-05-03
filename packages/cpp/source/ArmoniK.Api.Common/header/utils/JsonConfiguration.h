#pragma once
/**
 * @file JsonConfiguration.h
 * @brief Definition of a JSON configuration class that inherits from IConfiguration.
 */

#include "utils/IConfiguration.h"

namespace armonik::api::common::utils
{
  /**
   * @class JsonConfiguration
   * @brief JSON configuration class that inherits from IConfiguration.
   */
  class JsonConfiguration : public IConfiguration
  {
  public:
    /**
     * @brief Constructor that takes a JSON string.
     * @param string JSON string to be used for configuration.
     */
    explicit JsonConfiguration(const std::string& string)
    {
    }

    /**
     * @brief Get the value associated with the given key.
     * @param string Key to look up.
     * @return The value associated with the key, as a string.
     */
    [[nodiscard]] std::string get(const std::string& string) const override
    {
      return "";
    }

    /**
     * @brief Set the value associated with the given key.
     * @param string Key to set the value for.
     * @param value Value to set for the key.
     */
    void set(const std::string& string, const std::string& value) override
    {
    }

    /**
     * @brief Set the values from another IConfiguration object.
     * @param other IConfiguration object to copy values from.
     */
    void set(const IConfiguration& other) override
    {
    }
  };
}
