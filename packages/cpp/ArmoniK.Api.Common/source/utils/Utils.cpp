#include "utils/Utils.h"
#include <absl/strings/str_split.h>
#include <cmath>
#include <iomanip>

namespace armonik {
namespace api {
namespace common {
namespace utils {

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
  auto splitted = absl::StrSplit(timespan, ':');
  std::vector<std::string> sections(splitted.begin(), splitted.end());
  long days = 0, hours, minutes, seconds;
  if (sections.size() != 3) {
    throw std::invalid_argument("timespan is not of the format [-][d.]hh:mm:ss[.fffffffff]");
  }
  // Split the days.hours
  auto subsplitted = absl::StrSplit(sections[0], '.');
  std::vector<std::string> subsplit(subsplitted.begin(), subsplitted.end());
  if (subsplit.size() > 2) {
    throw std::invalid_argument("timespan is not of the format [-][d.]hh:mm:ss[.fffffffff]");
  }
  // Sign is only present in the first section
  int sign = absl::StrContains(subsplit[0], '-') ? -1 : 1;
  if (subsplit.size() == 2) {
    days = std::strtol(subsplit[0].c_str(), nullptr, 10);
    hours = sign * std::strtol(subsplit[1].c_str(), nullptr, 10);
  } else {
    hours = std::strtol(subsplit[0].c_str(), nullptr, 10);
  }

  minutes = sign * std::strtol(sections[1].c_str(), nullptr, 10);
  subsplitted = absl::StrSplit(sections[2], '.');
  std::vector<std::string> subsplit_sec(subsplitted.begin(), subsplitted.end());
  if (subsplit_sec.size() > 2) {
    throw std::invalid_argument("timespan is not of the format [-][d.]hh:mm:ss[.fffffffff]");
  }
  int nanos = 0;
  seconds = sign * std::strtol(subsplit_sec[0].c_str(), nullptr, 10);
  if (subsplit_sec.size() == 2) {
    if (subsplit_sec[1].length() >= 9) {
      nanos = sign * (int)std::strtol(subsplit_sec[1].substr(0, 9).c_str(), nullptr, 10);
    } else {
      nanos = sign *
              (int)std::strtol((subsplit_sec[1] + std::string(9 - subsplit_sec[1].length(), '0')).c_str(), nullptr, 10);
    }
  }

  return duration_from_values(days, hours, minutes, seconds, nanos);
}

} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
