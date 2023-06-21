/**
 * @file
 * @brief Contains utility structures for string manipulationand JSON string creation
 */

#pragma once

#include <iomanip>
#include <sstream>

/** @namespace armonik::api::common::utils
 *  @brief A namespace for common utility structures used within the Armonik API
 */
namespace armonik::api::common::utils {
/** @struct string_tools
 *  @brief A structure containing static string utility methods
 */
struct string_tools {
  /** @brief Escapes special characters in a JSON string
   *  @param input The input string
   *  @return A JSON-safe escaped string
   */
  static inline auto escape_json(const std::string &input) -> std::string {
    std::ostringstream output;

    for (const char c : input) {
      switch (c) {
      case '\"':
        output << "\\\"";
        break;
      case '\\':
        output << "\\\\";
        break;
      case '\b':
        output << "\\b";
        break;
      case '\f':
        output << "\\f";
        break;
      case '\n':
        output << "\\n";
        break;
      case '\t':
        output << "\\t";
        break;
      default:
        if (c < '\x20') {
          output << "\\u" << std::hex << std::setw(4) << std::setfill('0') << static_cast<int>(c);
        } else {
          output << c;
        }
      }
    }

    return output.str();
  }
};

/** @struct json_string
 *  @brief A structure for creating JSON-safe string objects
 */
struct json_string {
  json_string() = default;

  /** @brief Constructs a JSON-safe string object from a null-terminated char array
   *  @param value A pointer to a null-terminated char array
   */
  explicit json_string(const char *value) : str_val(value == nullptr ? "" : value) {}

  /** @brief Constructs a JSON-safe string object from a std::string
   *  @param value A std::string object
   */
  explicit json_string(const std::string &value) : str_val(value) {}

  /** @brief Constructs a JSON-safe string object from an rvalue std::string
   *  @param value An rvalue std::string object
   */
  explicit json_string(std::string &&value) : str_val(value) {}

  /** @brief Constructs a JSON-safe string object from an rvalue uint8_t
   *  @param value An rvalue uint8_t object
   */
  explicit json_string(uint8_t &&value) : str_val(std::to_string(value)) {}

  /** @brief Constructs a JSON-safe string object from an rvalue uint32_t
   *  @param value An rvalue uint32_t object
   */
  explicit json_string(uint32_t &&value) : str_val(std::to_string(value)) {}

  /** @brief Constructs a JSON-safe string object from an rvalue uint64_t
   *  @param value An rvalue uint64_t object
   */
  explicit json_string(uint64_t &&value) : str_val(std::to_string(value)) {}

  /** @brief Constructs a JSON-safe string object from an rvalue int8_t
   *  @param value An rvalue int8_t object
   */
  explicit json_string(int8_t &&value) : str_val(std::to_string(value)) {}

  /** @brief Constructs a JSON
   ** @brief Constructs a JSON-safe string object from an rvalue int32_t
   *  @param value An rvalue int32_t object
   */
  explicit json_string(int32_t &&value) : str_val(std::to_string(value)) {}

  /** @brief Constructs a JSON-safe string object from an rvalue int64_t
   *  @param value An rvalue int64_t object
   */
  explicit json_string(int64_t &&value_) : str_val(std::to_string(value_)) {}

  /** @brief Constructs a JSON-safe string object from a template parameter
   *  @tparam T The template parameter type
   *  @param value A value of type T
   */
  template <class T> json_string(T value) {
    std::ostringstream ss;
    ss << value;
    str_val = ss.str();
  }

  /** @brief Constructs a JSON-safe string object from a reference to a template parameter
   *  @tparam T The template parameter type
   *  @param value A const reference to a value of type T
   */
  template <class T> explicit json_string(const T &value) {
    std::ostringstream ss;
    ss << value;
    str_val = ss.str();
  }

  std::string str_val; ///< The JSON-safe string value
};
} // namespace armonik::api::common::utils
