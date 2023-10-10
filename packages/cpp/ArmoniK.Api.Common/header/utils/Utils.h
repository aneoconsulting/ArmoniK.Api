#pragma once

#include "google/protobuf/duration.pb.h"

namespace armonik {
namespace api {
namespace common {
namespace utils {
/**
 * Creates a duration from the given values
 * @param days Days
 * @param hours Hours
 * @param minutes Minutes
 * @param seconds Seconds
 * @param nanoseconds Nanoseconds
 * @return Duration with the right value
 * @note Make sure that the resulting number of seconds and the nanoseconds are of the same sign for the duration to be
 * valid
 */
::google::protobuf::Duration duration_from_values(long long int days = 0, long long int hours = 0,
                                                  long long int minutes = 0, long long int seconds = 0,
                                                  int nanoseconds = 0);

/**
 * Creates a duration from timespan string
 * @param timespan string with format [-][d.]hh:mm:ss[.fffffffff]
 * @return Duration in accordance with timespan
 */
::google::protobuf::Duration duration_from_timespan(const std::string &timespan);
} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
