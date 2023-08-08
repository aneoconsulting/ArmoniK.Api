#pragma once
/**
 * @file logger.h
 */

#include <functional>
#include <memory>
#include <string_view>

#include "context.h"
#include "level.h"
#include "local_logger.h"

namespace API_COMMON_NAMESPACE::logger {

class IWriter;
class IFormatter;

class Logger {
private:
  std::unique_ptr<IWriter> writer_;
  std::unique_ptr<IFormatter> formatter_;
  Context global_context_;
  std::map<std::string, std::function<std::string()>> local_context_generator_;
  Level level_;

public:
  Logger(std::unique_ptr<IWriter> writer, std::unique_ptr<IFormatter> formatter, Level level = Level::Info);
  Logger(const Logger &) = delete;
  Logger(Logger &&) noexcept = default;
  Logger &operator=(const Logger &) = delete;
  Logger &operator=(Logger &&) noexcept = default;
  ~Logger();

public:
  void set_level(Level level) noexcept { level_ = level; }
  Level get_level() const noexcept { return level_; }

  void global_context_add(std::string key, std::string value);
  const std::string &global_context_get(const std::string &key) const;
  void global_context_remove(const std::string &key);

  void local_context_generator_add(std::string key, std::function<std::string()> value_generator);
  const std::function<std::string()> &local_context_generator_get(const std::string &key) const;
  void local_context_generator_remove(const std::string &key);

  LocalLogger local(Context local_context = {});

public:
  void log(Level level, std::string_view message, const Context &message_context = {});
  void verbose(std::string_view message, const Context &message_context = {}) {
    log(Level::Verbose, message, message_context);
  }
  void debug(std::string_view message, const Context &message_context = {}) {
    log(Level::Debug, message, message_context);
  }
  void info(std::string_view message, const Context &message_context = {}) {
    log(Level::Info, message, message_context);
  }
  void warning(std::string_view message, const Context &message_context = {}) {
    log(Level::Warning, message, message_context);
  }
  void error(std::string_view message, const Context &message_context = {}) {
    log(Level::Error, message, message_context);
  }
  void fatal(std::string_view message, const Context &message_context = {}) {
    log(Level::Fatal, message, message_context);
  }
};
} // namespace API_COMMON_NAMESPACE::logger
