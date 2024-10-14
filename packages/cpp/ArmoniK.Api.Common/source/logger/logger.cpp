
#include "logger/context.h"
#include "logger/formatter.h"
#include "logger/local_logger.h"
#include "logger/writer.h"

#include "logger/logger.h"

namespace armonik {
namespace api {
namespace common {
namespace logger {

namespace {
// Empty string to return when key is not found
const std::string empty_string;
// Empty string generator when key is not found
const std::function<std::string()> empty_func = []() { return std::string(); };
} // namespace

// Construct a Logger
Logger::Logger(std::unique_ptr<IWriter> writer, std::unique_ptr<IFormatter> formatter, Level level)
    : ILogger(level), writer_(std::move(writer)), formatter_(std::move(formatter)) {}

// Default destructor
Logger::~Logger() = default;

// Add a new global context entry
void Logger::global_context_add(std::string key, std::string value) {
  global_context_[std::move(key)] = std::move(value);
}
// Get Value of a global context entry
const std::string &Logger::global_context_get(const std::string &key) const {
  // operator[] is not const on map
  // Use find instead
  auto it = global_context_.find(key);
  if (it == global_context_.end()) {
    return empty_string;
  }

  return it->second;
}
// Remove an entry from the global context
void Logger::global_context_remove(const std::string &key) { global_context_.erase(key); }

// Add a new local context generator entry
void Logger::local_context_generator_add(std::string key, std::function<std::string()> value_generator) {
  local_context_generator_[std::move(key)] = std::move(value_generator);
}
// Get Value generator of for a local context entry
const std::function<std::string()> &Logger::local_context_generator_get(const std::string &key) const {
  // operator[] is not const on map
  // Use find instead
  auto it = local_context_generator_.find(key);
  if (it == local_context_generator_.end()) {
    return empty_func;
  }

  return it->second;
}
// Remove a generator entry from the local context
void Logger::local_context_generator_remove(const std::string &key) { local_context_generator_.erase(key); }

// Create a logger with a local context that references this logger
LocalLogger Logger::local(Context local_context) const {
  // Populate local context from generator
  for (auto &&kg : local_context_generator_) {
    local_context[kg.first] = kg.second();
  }

  return LocalLogger(writer_.get(), formatter_.get(), &global_context_, std::move(local_context), level_);
}

// ILogger::log()
void Logger::log(Level level, absl::string_view message, const Context &message_context) {
  if (level < level_) {
    return;
  }
  local().log(level, message, message_context);
}

// Interface destructor
ILogger::~ILogger() = default;
} // namespace logger
} // namespace common
} // namespace api
} // namespace armonik
