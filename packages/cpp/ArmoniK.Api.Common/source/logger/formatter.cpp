#include <chrono>

#include <fmt/chrono.h>
#include <fmt/color.h>
#include <fmt/compile.h>
#include <fmt/format.h>

#include "logger/context.h"
#include "logger/level.h"

#include "logger/formatter.h"

namespace API_COMMON_NAMESPACE::logger {
/**
 * @brief Formatter for CLEF (Compact Log Event Format)
 */
class ClefFormatter : public IFormatter {
public:
  /**
   * @copydoc IFormatter::format()
   */
  std::string format(Level level, std::string_view message, const Context &global_context, const Context &local_context,
                     const Context &message_context) override {
    // Buffer to store the formatted string
    std::string buf;
    auto out = std::back_inserter(buf);

    // Get current time
    auto time = std::chrono::system_clock::now();
    auto ns = std::chrono::duration_cast<std::chrono::nanoseconds>(time.time_since_epoch()).count() % 1'000'000'000;

    // Format message with timestamp and level
    fmt::format_to(out, R"({{"@t": "{:%FT%T}.{:09}Z", "@mt": {:?}, "@l": {:?})", fmt::gmtime(time), ns, message,
                   level_name(level));

    // Add contexts to the formatted message
    for (auto context : {&global_context, &local_context, &message_context}) {
      for (auto &[key, val] : *context) {
        fmt::format_to(out, ", {:?}: {:?}", key, val);
      }
    }

    // Close Json
    buf.append("}");

    return buf;
  }
};

/**
 * @brief Formatter for a plain text format.
 */
class PlainFormatter : public IFormatter {
private:
  bool styling_ = false;

public:
  /**
   * @brief Construct a plain text formatter.
   * @param styling Whether terminal styling should be applied.
   */
  PlainFormatter(bool styling) : styling_(styling) {}

public:
  /**
   * @copydoc IFormatter::format()
   */
  std::string format(Level level, std::string_view message, const Context &global_context, const Context &local_context,
                     const Context &message_context) override {
    // Buffer to store the formatted string
    std::string buf;
    auto out = std::back_inserter(buf);

    // Get current time
    auto time = std::chrono::system_clock::now();
    auto ns = std::chrono::duration_cast<std::chrono::nanoseconds>(time.time_since_epoch()).count() % 1'000'000'000;

    // Style for the message
    auto message_style = styling_ ? fmt::emphasis::bold : fmt::text_style{};

    // Format message with timestamp and level
    fmt::format_to(out, "{:%FT%T}.{:09}z\t[{}]\t{}", fmt::gmtime(time), ns, level_name(level),
                   fmt::styled(message, message_style));

    // Add contexts to the formatted message
    for (auto context : {&global_context, &local_context, &message_context}) {
      for (auto &[key, val] : *context) {
        fmt::format_to(out, "\t{}={}", key, val);
      }
    }

    return buf;
  }
};

std::unique_ptr<IFormatter> formatter_clef() { return std::make_unique<ClefFormatter>(); }
std::unique_ptr<IFormatter> formatter_clef(bool styling) { return std::make_unique<PlainFormatter>(styling); }

// Interface destructor
IFormatter::~IFormatter() = default;
} // namespace API_COMMON_NAMESPACE::logger
