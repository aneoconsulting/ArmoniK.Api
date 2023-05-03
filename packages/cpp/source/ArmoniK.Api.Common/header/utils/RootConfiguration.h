/**
 * @file RootConfiguration.h
 * @brief Definition of a root configuration class that inherits from IConfiguration.
 */

#pragma once
#include "utils/IConfiguration.h"

namespace armonik::api::common::utils
{
  /**
   * @class RootConfiguration
   * @brief Root configuration class that inherits from IConfiguration.
   */
  class RootConfiguration : public IConfiguration
  {
  public:
    /**
     * @brief Default constructor.
     */
    RootConfiguration()
      = default;

    /**
     * @brief Get the value associated with the given key.
     * @param key Key to look up.
     * @return The value associated with the key, as a string.
     */
    [[nodiscard]] std::string get(const std::string& key) const override
    {
      auto pair = options_.find(key);

      return (pair != options_.end()) ? (*pair).second : "";
    }

    /**
     * @brief Set the value associated with the given key.
     * @param key Key to set the value for.
     * @param value Value to set for the key.
     */
    void set(const std::string& key, const std::string& value) override
    {
      options_[key] = value;
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
