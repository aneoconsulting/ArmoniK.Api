#pragma once
/**
 * @file fwd.h
 * @brief Forward declarations for logger classes.
 */

namespace API_COMMON_NAMESPACE::logger {
class ILogger;
class IFormatter;
class IWriter;

class Context;
class Logger;
class LocalLogger;

enum class Level;
} // namespace API_COMMON_NAMESPACE::logger
