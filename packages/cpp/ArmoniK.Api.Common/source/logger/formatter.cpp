#include <chrono>

#include <fmt/chrono.h>
#include <fmt/color.h>
#include <fmt/compile.h>
#include <fmt/format.h>

#include "logger/formatter.h"

namespace API_COMMON_NAMESPACE::logger {

class ClefFormatter : public IFormatter {
public:
  std::string format(Level level, std::string_view message, const Context &global_context, const Context &local_context,
                     const Context &message_context) override {
    std::string buf;
    auto out = std::back_inserter(buf);

    auto time = std::chrono::system_clock::now();
    auto ns = std::chrono::duration_cast<std::chrono::nanoseconds>(time.time_since_epoch()).count() % 1'000'000'000;

    fmt::format_to(out, FMT_COMPILE(R"({{"@t": "{:%FT%T}.{:09}Z", "@mt": {:?}, "@l": {:?})"), fmt::gmtime(time), ns,
                   message, level_name(level));

    for (auto context : {&global_context, &local_context, &message_context}) {
      for (auto &[key, val] : *context) {
        fmt::format_to(out, FMT_COMPILE(", {:?}: {:?}"), key, val);
      }
    }

    buf.append("}");

    return buf;
  }
};

class PlainFormatter : public IFormatter {
private:
  bool styling_ = false;

public:
  PlainFormatter(bool styling) : styling_(styling) {}

public:
  std::string format(Level level, std::string_view message, const Context &global_context, const Context &local_context,
                     const Context &message_context) override {
    std::string buf;
    auto out = std::back_inserter(buf);

    auto time = std::chrono::system_clock::now();
    auto ns = std::chrono::duration_cast<std::chrono::nanoseconds>(time.time_since_epoch()).count() % 1'000'000'000;

    auto message_style = styling_ ? fmt::emphasis::bold : fmt::text_style{};

    fmt::format_to(out, FMT_COMPILE("{:%FT%T}.{:09}z\t[{}]\t{}"), fmt::gmtime(time), ns, level_name(level),
                   fmt::styled(message, message_style));

    for (auto context : {&global_context, &local_context, &message_context}) {
      for (auto &[key, val] : *context) {
        fmt::format_to(out, FMT_COMPILE("\t{}={}"), key, val);
      }
    }

    return buf;
  }
};

std::unique_ptr<IFormatter> formatter_clef() { return std::make_unique<ClefFormatter>(); }
std::unique_ptr<IFormatter> formatter_clef(bool styling) { return std::make_unique<PlainFormatter>(styling); }
} // namespace API_COMMON_NAMESPACE::logger
