#pragma once
/**
 * @file level.h
 * @brief Logging levels.
 */

#include <utils/string_view.h>

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
// C++11 constexpr forbids switch: https://en.cppreference.com/w/cpp/language/constexpr (C++11 notes)
constexpr string_view level_name(Level level) noexcept {
  return level == Level::Verbose ? string_view("Verbose") :
         level == Level::Debug   ? string_view("Debug")   :
         level == Level::Info    ? string_view("Info")     :
         level == Level::Warning ? string_view("Warning")  :
         level == Level::Error   ? string_view("Error")    :
         level == Level::Fatal   ? string_view("Fatal")    :
                                   string_view("Unknown");
}
} // namespace logger
} // namespace common
} // namespace api
} // namespace armonik
