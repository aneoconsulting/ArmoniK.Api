#pragma once
/**
 * @file fwd.h
 * @brief Forward declarations for logger classes.
 */

namespace armonik {
namespace api {
namespace common {
namespace logger {
class ILogger;
class IFormatter;
class IWriter;

class Context;
class Logger;
class LocalLogger;

enum class Level;
} // namespace logger
} // namespace common
} // namespace api
} // namespace armonik
