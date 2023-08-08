
#include "logger/context.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "logger/local_logger.h"

namespace API_COMMON_NAMESPACE::logger {

namespace {
// Empty string to return when key is not found
static const std::string empty_string{};
// Empty string generator when key is not found
static const std::function<std::string()> empty_func = []() { return std::string(); };
} // namespace

// Construct a LocalLogger (called from Logger)
LocalLogger::LocalLogger(IWriter *writer, IFormatter *formatter, const Context *global_context, Context local_context,
                         Level level)
    : ILogger(level), writer_(writer), formatter_(formatter), global_context_(global_context),
      local_context_(std::move(local_context)) {}

// Default destructor
LocalLogger::~LocalLogger() = default;

// Add a new context entry
void LocalLogger::context_add(std::string key, std::string value) { local_context_[std::move(key)] = std::move(value); }

// Get Value of a context entry
const std::string &LocalLogger::context_get(const std::string &key) const {
  // operator[] is not const on map
  // Use find instead
  auto it = local_context_.find(key);
  if (it == local_context_.end()) {
    return empty_string;
  }

  return it->second;
}

// Remove an entry from the context
void LocalLogger::context_remove(const std::string &key) { local_context_.erase(key); }

// Write a new message to the log
void LocalLogger::log(Level level, std::string_view message, const Context &message_context) {
  if (level < level_) {
    return;
  }

  auto formatted = formatter_->format(level_, message, *global_context_, local_context_, message_context);
  writer_->write(level_, formatted);
}
} // namespace API_COMMON_NAMESPACE::logger
