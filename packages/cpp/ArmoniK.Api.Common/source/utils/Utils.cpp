#include "utils/Utils.h"
#include <cmath>
#include <iomanip>
#include <utils/string_view.h>

namespace armonik {
namespace api {
namespace common {
namespace utils {

namespace {
// strtol needs a null-terminated buffer, which string_view doesn't carry, so parsing a
// numeric field still requires a small std::string allocation. Localized here so the
// duration_from_timespan body isn't repeating static_cast<std::string>(...).c_str().
long parse_long(string_view sv) { return std::strtol(std::string(sv).c_str(), nullptr, 10); }
} // namespace

::google::protobuf::Duration duration_from_values(long long int days, long long int hours, long long int minutes,
                                                  long long int seconds, int nanoseconds) {
  ::google::protobuf::Duration duration;
  duration.set_seconds(days * 86400 + 3600 * hours + 60 * minutes + seconds);
  duration.set_nanos(nanoseconds);
  return duration;
}

/**
 * Creates a duration from timespan string
 * @param timespan string with format [-][d.]hh:mm:ss[.fffffffff]
 * @return Duration in accordance with timespan
 */
::google::protobuf::Duration duration_from_timespan(const std::string &timespan) {
  std::vector<string_view> sections = string_view(timespan).split(':');
  long days = 0, hours, minutes, seconds;
  if (sections.size() != 3) {
    throw std::invalid_argument("timespan is not of the format [-][d.]hh:mm:ss[.fffffffff]");
  }
  // Split the days.hours
  std::vector<string_view> subsplit = sections[0].split('.');
  if (subsplit.size() > 2) {
    throw std::invalid_argument("timespan is not of the format [-][d.]hh:mm:ss[.fffffffff]");
  }
  // Sign is only present in the first section
  int sign = subsplit[0].contains('-') ? -1 : 1;
  if (subsplit.size() == 2) {
    days = parse_long(subsplit[0]);
    hours = sign * parse_long(subsplit[1]);
  } else {
    hours = parse_long(subsplit[0]);
  }

  minutes = sign * parse_long(sections[1]);
  std::vector<string_view> subsplit_sec = sections[2].split('.');
  if (subsplit_sec.size() > 2) {
    throw std::invalid_argument("timespan is not of the format [-][d.]hh:mm:ss[.fffffffff]");
  }
  int nanos = 0;
  seconds = sign * parse_long(subsplit_sec[0]);
  if (subsplit_sec.size() == 2) {
    if (subsplit_sec[1].size() >= 9) {
      nanos = sign * (int)parse_long(subsplit_sec[1].substr(0, 9));
    } else {
      nanos = sign *
              (int)parse_long(static_cast<std::string>(subsplit_sec[1]) + std::string(9 - subsplit_sec[1].size(), '0'));
    }
  }

  return duration_from_values(days, hours, minutes, seconds, nanos);
}

} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
