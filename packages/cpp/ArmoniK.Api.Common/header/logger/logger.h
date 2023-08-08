#pragma once
/**
 * @file logger.h
 */

#include <functional>
#include <memory>
#include <string_view>

#include "base.h"
#include "context.h"
#include "fwd.h"
#include "level.h"
#include "local_logger.h"

namespace API_COMMON_NAMESPACE::logger {
/**
 * @brief Default Logger.
 */
class Logger : public ILogger {
private:
  std::unique_ptr<IWriter> writer_;
  std::unique_ptr<IFormatter> formatter_;
  Context global_context_;
  std::map<std::string, std::function<std::string()>> local_context_generator_;

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
   * @brief Destructor.
   */
  ~Logger();

public:
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
   * @copydoc ILogger::log()
   * @details Thread-safe.
   */
  void log(Level level, std::string_view message, const Context &message_context = {}) override;
};
} // namespace API_COMMON_NAMESPACE::logger
