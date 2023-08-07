#pragma once
/**
 * @file logger.h
 */

#include <memory>
#include <string_view>

#include "context.h"
#include "level.h"

namespace API_COMMON_NAMESPACE::logger {

class IFormatter {
public:
  virtual ~IFormatter();

  virtual std::string format(std::string_view message, const Context &global_context, const Context &local_context,
                             const Context &message_context);
};

std::unique_ptr<IFormatter> clef_formatter();
std::unique_ptr<IFormatter> plain_formatter();

} // namespace API_COMMON_NAMESPACE::logger
