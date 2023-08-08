#pragma once
/**
 * @file local_logger.h
 * @brief Logger with a local context.
 */

#include <string_view>

#include "context.h"
#include "fwd.h"
#include "level.h"

namespace API_COMMON_NAMESPACE::logger {

/**
 * @class LocalLogger
 * @brief Logger with a local context.
 */
class LocalLogger {
private:
  IWriter *writer_;
  IFormatter *formatter_;
  const Context *global_context_;
  Context local_context_;
  Level level_;

private:
  friend class Logger;
  LocalLogger(IWriter *writer, IFormatter *formatter, const Context *global_context, Context local_context,
              Level level);

public:
  LocalLogger() = delete;
  LocalLogger(const LocalLogger &) = delete;
  LocalLogger &operator=(const LocalLogger &) = delete;
  /**
   * @brief Move constructor.
   * @param other Local logger to move.
   */
  LocalLogger(LocalLogger &&other) noexcept = default;
  /**
   * @brief Move assignment operator.
   * @param other Local logger to move.
   * @return This.
   */
  LocalLogger &operator=(LocalLogger &&other) noexcept = default;
  /**
   * @brief Destructor.
   */
  ~LocalLogger();

public:
  /**
   * @brief Set the logging level.
   * @param level Logging level.
   */
  void set_level(Level level) noexcept { level_ = level; }
  /**
   * @brief Get the current logging level.
   * @return The current logging level.
   */
  Level get_level() const noexcept { return level_; }

  /**
   * @brief Add a new context entry.
   * @param key Name of the entry.
   * @param value Value of the entry.
   */
  void context_add(std::string key, std::string value);
  /**
   * @brief Get Value of a context entry.
   * @param key Name of the entry to fetch.
   * @return Value of the entry.
   */
  const std::string &context_get(const std::string &key) const;
  /**
   * @brief Remove an entry from the context.
   * @param key Name of the entry to remove.
   */
  void context_remove(const std::string &key);

public:
  /**
   * @brief Write a new message to the log.
   * @param level Logging level to use for this message.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void log(Level level, std::string_view message, const Context &message_context = {});
  /**
   * @brief Write a new message to the log with verbose log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void verbose(std::string_view message, const Context &message_context = {}) {
    log(Level::Verbose, message, message_context);
  }
  /**
   * @brief Write a new message to the log with debug log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void debug(std::string_view message, const Context &message_context = {}) {
    log(Level::Debug, message, message_context);
  }
  /**
   * @brief Write a new message to the log with info log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void info(std::string_view message, const Context &message_context = {}) {
    log(Level::Info, message, message_context);
  }
  /**
   * @brief Write a new message to the log with warning log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void warning(std::string_view message, const Context &message_context = {}) {
    log(Level::Warning, message, message_context);
  }
  /**
   * @brief Write a new message to the log with error log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void error(std::string_view message, const Context &message_context = {}) {
    log(Level::Error, message, message_context);
  }
  /**
   * @brief Write a new message to the log with fatal log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void fatal(std::string_view message, const Context &message_context = {}) {
    log(Level::Fatal, message, message_context);
  }
};
} // namespace API_COMMON_NAMESPACE::logger
