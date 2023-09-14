#pragma once

#include <absl/strings/string_view.h>
/**
 * @file level.h
 * @brief Logging levels.
 */

namespace armonik {
namespace api {
namespace common {
namespace logger {
/**
 * @enum Level
 * @brief Logging Level datatype.
 */
enum class Level {
  Verbose = 0,
  Debug = 1,
  Info = 2,
  Warning = 3,
  Error = 4,
  Fatal = 5,
};

/**
 * @brief Convert a log level into a static string view.
 * @param level Log level to convert.
 * @return String view representing the log level.
 */
constexpr absl::string_view level_name(Level level) {
  switch (level) {
  case Level::Verbose:
    return "Verbose";
  case Level::Debug:
    return "Debug";
  case Level::Info:
    return "Info";
  case Level::Warning:
    return "Warning";
  case Level::Error:
    return "Error";
  case Level::Fatal:
    return "Fatal";
  default:
    return "Unknown";
  }
}
} // namespace logger
} // namespace common
} // namespace api
} // namespace armonik
