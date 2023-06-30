#pragma once

#include "utils/StringsUtils.h"
#include <vector>

/// @namespace armonik::api::common::serilog
/// @brief A namespace for serilog functionality in the Armonik API
namespace armonik::api::common::serilog {
/// @typedef serilog_properties_pair_t
/// @brief A pair containing a string as a key and a json_string as a value for serilog properties
using serilog_properties_pair_t = std::pair<std::string, utils::json_string>;

/// @typedef serilog_properties_vector_t
/// @brief A vector of serilog_properties_pair_t for storing multiple serilog properties
using serilog_properties_vector_t = std::vector<serilog_properties_pair_t>;

/// @enum logging_level
/// @brief An enumeration representing the different logging levels for serilog
enum logging_level {
  verbose = 0,
  ///< Verbose logging level (lowest)
  debug = 1,
  ///< Debug logging level
  info = 2,
  ///< Information logging level
  warning = 3,
  ///< Warning logging level
  error = 4,
  ///< Error logging level
  fatal = 5 ///< Fatal logging level (highest)
};

enum logging_format {
  CONSOLE = 0,
  SEQ = 1,
};
} // namespace armonik::api::common::serilog
