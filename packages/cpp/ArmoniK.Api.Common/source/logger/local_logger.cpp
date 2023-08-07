#include "logger/local_logger.h"
#include "logger/context.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

namespace API_COMMON_NAMESPACE::logger {

namespace {
static const std::string empty_string{};
static const std::function<std::string()> empty_func = []() { return std::string(); };
} // namespace

LocalLogger::LocalLogger(IWriter *writer, IFormatter *formatter, Context *global_context, Context local_context,
                         Level level)
    : writer_(writer), formatter_(formatter), global_context_(global_context), local_context_(std::move(local_context)),
      level_(level) {}
LocalLogger::~LocalLogger() = default;

void LocalLogger::context_add(std::string key, std::string value) { local_context_[std::move(key)] = std::move(value); }
const std::string &LocalLogger::context_get(const std::string &key) const {
  auto it = local_context_.find(key);
  if (it == local_context_.end()) {
    return empty_string;
  }
  return it->second;
}
void LocalLogger::context_remove(const std::string &key) { local_context_.erase(key); }

void LocalLogger::log(Level level, std::string_view message, const Context &message_context = {}) {
  if (level < level_) {
    return;
  }
  auto formatted = formatter_->format(message, *global_context_, local_context_, message_context);
  writer_->write(formatted);
}
} // namespace API_COMMON_NAMESPACE::logger
