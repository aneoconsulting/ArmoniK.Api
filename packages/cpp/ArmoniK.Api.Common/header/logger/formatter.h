#pragma once
/**
 * @file formatter.h
 * @brief Formatter interface.
 */

#include <memory>
#include <string_view>

#include "fwd.h"
#include "level.h"

namespace armonik::api::common::logger {
/**
 * @interface IFormatter
 * @brief Formatter interface to use by a logger.
 */
class IFormatter {
public:
  /**
   * @brief Destructor.
   */
  virtual ~IFormatter();

  /**
   * @brief Format log message with context.
   * @param level Log level to use for this message.
   * @param message Message to write in the log.
   * @param global_context Context that is set globally within the logger.
   * @param local_context Context set locally.
   * @param message_context Context specific to this very message.
   * @return The formatted message.
   */
  virtual std::string format(Level level, std::string_view message, const Context &global_context,
                             const Context &local_context, const Context &message_context) = 0;
};

/**
 * @brief Get a formatter for the CLEF format.
 * @return Pointer to the formatter.
 */
std::unique_ptr<IFormatter> formatter_clef();
/**
 * @brief Get a formatter for plain text format.
 * @param styling Whether terminal styling should be applied.
 * @return Pointer to the formatter.
 */
std::unique_ptr<IFormatter> formatter_plain(bool styling = false);
} // namespace armonik::api::common::logger
