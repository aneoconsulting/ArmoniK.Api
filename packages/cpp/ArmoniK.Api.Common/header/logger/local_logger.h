#pragma once
/**
 * @file logger.h
 */

#include <functional>
#include <memory>
#include <string_view>

#include "context.h"
#include "formatter.h"
#include "level.h"
#include "writer.h"

namespace API_COMMON_NAMESPACE::logger {

class Logger;

class LocalLogger {
private:
  IWriter *writer_;
  IFormatter *formatter_;
  Context *global_context_;
  Context local_context_;
  Level level_;

private:
  friend class Logger;
  LocalLogger(IWriter *writer, IFormatter *formatter, Context *global_context, Context local_context, Level level);

public:
  LocalLogger(const LocalLogger &) = delete;
  LocalLogger(LocalLogger &&) noexcept = default;
  LocalLogger &operator=(const LocalLogger &) = delete;
  LocalLogger &operator=(LocalLogger &&) noexcept = default;
  ~LocalLogger();

public:
  void set_level(Level level) noexcept { level_ = level; }
  Level get_level() const noexcept { return level_; }

  void context_add(std::string key, std::string value);
  const std::string &context_get(const std::string &key) const;
  void context_remove(const std::string &key);

public:
  void log(Level level, std::string_view message, const Context &message_context = {});
  void verbose(std::string_view message, const Context &message_context = {}) {
    log(Verbose, message, message_context);
  }
  void debug(std::string_view message, const Context &message_context = {}) { log(Debug, message, message_context); }
  void info(std::string_view message, const Context &message_context = {}) { log(Info, message, message_context); }
  void warning(std::string_view message, const Context &message_context = {}) {
    log(Warning, message, message_context);
  }
  void error(std::string_view message, const Context &message_context = {}) { log(Error, message, message_context); }
  void fatal(std::string_view message, const Context &message_context = {}) { log(Fatal, message, message_context); }
};
} // namespace API_COMMON_NAMESPACE::logger
