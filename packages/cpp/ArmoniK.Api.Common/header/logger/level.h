#pragma once
/**
 * @file level.h
 */

#include <string_view>

enum class Level {
  Verbose = 0,
  Debug = 1,
  Info = 2,
  Warning = 3,
  Error = 4,
  Fatal = 5,
};

std::string_view level_name(Level level) {
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
