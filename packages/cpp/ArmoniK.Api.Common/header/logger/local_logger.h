#pragma once
/**
 * @file local_logger.h
 * @brief Logger with a local context.
 */

#include <string_view>

#include "base.h"
#include "context.h"
#include "fwd.h"
#include "level.h"

namespace API_COMMON_NAMESPACE::logger {

/**
 * @class LocalLogger
 * @brief Logger with a local context.
 */
class LocalLogger : ILogger {
private:
  IWriter *writer_;
  IFormatter *formatter_;
  const Context *global_context_;
  Context local_context_;

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
   * @copydoc ILogger::log()
   */
  void log(Level level, std::string_view message, const Context &message_context = {}) override;
};
} // namespace API_COMMON_NAMESPACE::logger
