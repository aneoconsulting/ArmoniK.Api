#pragma once
/**
 * @file logger.h
 */

#include <functional>
#include <memory>
#include <string_view>

#include "context.h"
#include "fwd.h"
#include "level.h"
#include "local_logger.h"

namespace API_COMMON_NAMESPACE::logger {

class IWriter;
class IFormatter;

/**
 * @brief Logger.
 */
class Logger {
private:
  std::unique_ptr<IWriter> writer_;
  std::unique_ptr<IFormatter> formatter_;
  Context global_context_;
  std::map<std::string, std::function<std::string()>> local_context_generator_;
  Level level_;

public:
  /**
   * @brief Construct a new logger with custom writer and formatter.
   * @param writer Writer used to write log messages.
   * @param formatter Formatter used to format log messages with context.
   * @param level Logging level to use for the logger.
   */
  Logger(std::unique_ptr<IWriter> writer, std::unique_ptr<IFormatter> formatter, Level level = Level::Info);

  Logger() = delete;
  Logger(const Logger &) = delete;
  Logger &operator=(const Logger &) = delete;
  /**
   * @brief Move constructor.
   * @param other Logger to move.
   */
  Logger(Logger &&other) noexcept = default;
  /**
   * @brief Move assignment operator.
   * @param other Logger to move.
   * @return This.
   */
  Logger &operator=(Logger &&other) noexcept = default;
  /**
   * @brief Destrcutor.
   */
  ~Logger();

public:
  /**
   * @brief Set the logging level.
   * @param level Logging level.
   * @attention Not thread safe.
   */
  void set_level(Level level) noexcept { level_ = level; }
  /**
   * @brief Get the current logging level.
   * @return The current logging level.
   * @attention Not thread safe.
   */
  Level get_level() const noexcept { return level_; }

  /**
   * @brief Add a new global context entry.
   * @param key Name of the entry.
   * @param value Value of the entry.
   * @attention Not thread safe.
   */
  void global_context_add(std::string key, std::string value);
  /**
   * @brief Get Value of a global context entry.
   * @param key Name of the entry to fetch.
   * @return Value of the entry.
   * @attention Not thread safe.
   */
  const std::string &global_context_get(const std::string &key) const;
  /**
   * @brief Remove an entry from the global context.
   * @param key Name of the entry to remove.
   * @attention Not thread safe.
   */
  void global_context_remove(const std::string &key);

  /**
   * @brief Add a new local context generator entry.
   * @param key Name of the entry.
   * @param value_generator Value generator for the entry.
   * @attention Not thread safe.
   */
  void local_context_generator_add(std::string key, std::function<std::string()> value_generator);
  /**
   * @brief Get Value generator of for a local context entry.
   * @param key Name of the entry to fetch.
   * @return Value generator for the entry.
   * @attention Not thread safe.
   */
  const std::function<std::string()> &local_context_generator_get(const std::string &key) const;
  /**
   * @brief Remove a generator entry from the local context.
   * @param key Name of the entry to remove.
   * @attention Not thread safe.
   */
  void local_context_generator_remove(const std::string &key);

  /**
   * @brief Create a logger with a local context that references this logger.
   * @param local_context Local context to use.
   */
  LocalLogger local(Context local_context = {}) const;

public:
  /**
   * @brief Write a new message to the log.
   * @param level Logging level to use for this message.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void log(Level level, std::string_view message, const Context &message_context = {}) const;
  /**
   * @brief Write a new message to the log with verbose log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void verbose(std::string_view message, const Context &message_context = {}) const {
    log(Level::Verbose, message, message_context);
  }
  /**
   * @brief Write a new message to the log with debug log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void debug(std::string_view message, const Context &message_context = {}) const {
    log(Level::Debug, message, message_context);
  }
  /**
   * @brief Write a new message to the log with info log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void info(std::string_view message, const Context &message_context = {}) const {
    log(Level::Info, message, message_context);
  }
  /**
   * @brief Write a new message to the log with warning log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void warning(std::string_view message, const Context &message_context = {}) const {
    log(Level::Warning, message, message_context);
  }
  /**
   * @brief Write a new message to the log with error log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void error(std::string_view message, const Context &message_context = {}) const {
    log(Level::Error, message, message_context);
  }
  /**
   * @brief Write a new message to the log with fatal log level.
   * @param message Message to log.
   * @param message_context Context specific for this message.
   */
  void fatal(std::string_view message, const Context &message_context = {}) const {
    log(Level::Fatal, message, message_context);
  }
};
} // namespace API_COMMON_NAMESPACE::logger
