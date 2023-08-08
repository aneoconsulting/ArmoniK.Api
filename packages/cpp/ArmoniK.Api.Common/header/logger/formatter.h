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

  virtual std::string format(Level level, std::string_view message, const Context &global_context,
                             const Context &local_context, const Context &message_context);
};

std::unique_ptr<IFormatter> formatter_clef();
std::unique_ptr<IFormatter> formatter_clef(bool styling = false);

} // namespace API_COMMON_NAMESPACE::logger
