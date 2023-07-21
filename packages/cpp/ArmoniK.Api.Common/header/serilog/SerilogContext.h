#pragma once
#include "serilog_typedef.h"

/**
 * @brief This namespace encapsulates the Serilog logging system used in the Armonik API.
 */
namespace API_COMMON_NAMESPACE::serilog {
/**
 * @brief A struct representing a Serilog context with support for adding and retrieving properties.
 */
struct serilog_context {
public:
  /**
   * @brief Retrieves a reference to a Serilog properties pair at a given index.
   * @param index_ The index of the desired properties pair.
   * @return A constant reference to the properties pair at the specified index.
   */
  const serilog_properties_pair_t &operator[](size_t index) const { return _properties[index]; }

  /**
   * @brief Appends properties from another Serilog properties vector to this context.
   * @param other_ The other Serilog properties vector to append.
   */
  void append(const serilog_properties_vector_t &other) {
    if (other.empty())
      return;
    _properties.insert(_properties.end(), other.begin(), other.end());
  }

  /**
   * @brief Retrieves a reference to a Serilog properties pair at a given index.
   * @param index_ The index of the desired properties pair.
   * @return A reference to the properties pair at the specified index.
   */
  serilog_properties_pair_t &operator[](size_t index) { return _properties[index]; }

  /**
   * @brief Default constructor for the serilog_context struct.
   */
  serilog_context();

  /**
   * @brief Checks if the properties vector is empty.
   * @return True if the properties vector is empty, otherwise false.
   */
  [[nodiscard]] bool empty() const { return _properties.empty(); }

  /**
   * @brief Retrieves the number of properties pairs in the context.
   * @return The size of the properties vector.
   */
  [[nodiscard]] size_t size() const { return _properties.size(); }

  /**
   * @brief Constructor for the serilog_context struct with move semantics for parameters.
   * @param level The logging level for this context.
   * @param parameters A vector of Serilog properties pairs.
   * @param logger_name The name of the logger.
   */
  serilog_context(const logging_level level, serilog_properties_vector_t parameters, const char *logger_name)
      : level(level), logger_name(logger_name), _properties(std::move(parameters)) {}

  /**
   * @brief Adds a key-value pair to the Serilog properties vector.
   * @param key_ The key of the properties pair.
   * @param value_ The value of the properties pair.
   */
  void add(std::string key, utils::json_string value) { _properties.emplace_back(std::move(key), std::move(value)); }

  logging_level level; ///< The logging level for this context.
  const std::string logger_name;
  ///< The name of the logger associated with this context.
private:
  serilog_properties_vector_t _properties;
  ///< The vector of Serilog properties pairs.
};
} // namespace API_COMMON_NAMESPACE::serilog
