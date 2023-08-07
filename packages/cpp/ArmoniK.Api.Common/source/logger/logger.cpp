#include "logger/logger.h"
#include "logger/context.h"
#include "logger/formatter.h"
#include "logger/local_logger.h"
#include "logger/writer.h"

namespace API_COMMON_NAMESPACE::logger {

namespace {
static const std::string empty_string{};
static const std::function<std::string()> empty_func = []() { return std::string(); };
} // namespace

Logger::Logger(std::unique_ptr<IWriter> writer, std::unique_ptr<IFormatter> formatter, Level level = Info)
    : writer_(std::move(writer)), formatter_(std::move(formatter)), level_(level) {}
Logger::~Logger() = default;

void Logger::global_context_add(std::string key, std::string value) {
  global_context_[std::move(key)] = std::move(value);
}
const std::string &Logger::global_context_get(const std::string &key) const {
  auto it = global_context_.find(key);
  if (it == global_context_.end()) {
    return empty_string;
  }
  return it->second;
}
void Logger::global_context_remove(const std::string &key) { global_context_.erase(key); }

void Logger::local_context_generator_add(std::string key, std::function<std::string()> value_generator) {
  local_context_generator_[std::move(key)] = std::move(value_generator);
}
const std::function<std::string()> &Logger::local_context_generator_get(const std::string &key) const {
  auto it = local_context_generator_.find(key);
  if (it == local_context_generator_.end()) {
    return empty_func;
  }
  return it->second;
}
void Logger::local_context_generator_remove(const std::string &key) { local_context_generator_.erase(key); }

LocalLogger Logger::local(Context local_context = {}) {
  for (auto &&[key, generator] : local_context_generator_) {
    local_context[key] = generator();
  }
  return LocalLogger(writer_.get(), formatter_.get(), &global_context_, std::move(local_context), level_);
}

void Logger::log(Level level, std::string_view message, const Context &message_context = {}) {
  if (level < level_) {
    return;
  }
  local().log(level, message, message_context);
}
} // namespace API_COMMON_NAMESPACE::logger
