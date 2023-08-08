#pragma once
/**
 * @file context.h
 * @brief Logger context.
 */

#include <map>
#include <string>

namespace API_COMMON_NAMESPACE::logger {
/**
 * @class Context
 * @brief Logger context.
 */
class Context : public std::map<std::string, std::string> {
public:
  using std::map<std::string, std::string>::map;
  using std::map<std::string, std::string>::operator=;
};
} // namespace API_COMMON_NAMESPACE::logger
