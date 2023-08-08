#pragma once
/**
 * @file level.h
 */

enum Level {
  Verbose = 0,
  Debug = 1,
  Info = 2,
  Warning = 3,
  Error = 4,
  Fatal = 5,
};

std::string_view level_name(Level level) {
  switch (level) {
  case Verbose:
    return "Verbose";
  case Debug:
    return "Debug";
  case Info:
    return "Info";
  case Warning:
    return "Warning";
  case Error:
    return "Error";
  case Fatal:
    return "Fatal";
  default:
    return "Unknown";
  }
}
