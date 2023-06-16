/**
 * @file EnvConfiguration.h
 * @brief Header file for the EnvConfiguration class
 */

#include "utils/IConfiguration.h"

namespace armonik::api::common::utils
{
  /**
   * @class EnvConfiguration
   * @brief An implementation of IConfiguration that handles environment variables
   */
  class EnvConfiguration : public IConfiguration
  {
  public:
    /**
     * @brief Default constructor
     */
    EnvConfiguration()
      = default;

    /**
     * @brief Gets the value of an environment variable
     * @param string The name of the environment variable
     * @return The value of the environment variable, or an empty string if not found
     */
    [[nodiscard]] std::string get(const std::string& string) const override
    {
      std::string value = std::getenv(string.c_str());
      if (!value.empty())
      {
        return value;
      }else
      {
        throw std::runtime_error("Can't get server address !");
      }
      
    }

    /**
     * @brief Sets the value of an environment variable
     * @param string The name of the environment variable
     * @param value The value to set
     */
    void set(const std::string& string, const std::string& value) override
    {

    }

    /**
     * @brief Copies the values of another IConfiguration object into this one
     * @param other The IConfiguration object to copy from
     */
    void set(const IConfiguration& other) override
    {

    }
  };
}
