#pragma once
/**
 * @file context.h
 * @brief Logger context.
 */

#include <map>
#include <string>

namespace armonik {
namespace api {
namespace common {
namespace logger {
/**
 * @class Context
 * @brief Logger context.
 */
class Context : public std::map<std::string, std::string> {
public:
  using std::map<std::string, std::string>::map;
  using std::map<std::string, std::string>::operator=;
};
} // namespace logger
} // namespace common
} // namespace api
} // namespace armonik
