#pragma once
/**
 * @file fwd.h
 * @brief Forward declarations for logger classes.
 */

namespace armonik::api::common::logger {
class ILogger;
class IFormatter;
class IWriter;

class Context;
class Logger;
class LocalLogger;

enum class Level;
} // namespace armonik::api::common::logger
