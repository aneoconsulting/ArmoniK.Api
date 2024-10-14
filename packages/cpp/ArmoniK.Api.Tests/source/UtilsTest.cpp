#include <google/protobuf/duration.pb.h>
#include <gtest/gtest.h>
#include <sstream>

#include "utils/Utils.h"

struct timespan_test_case_from_string {
  ::google::protobuf::Duration expected;
  std::string value_to_test;
};

timespan_test_case_from_string genCaseForString(long seconds, int nanos, const std::string &str) {
  timespan_test_case_from_string value;
  value.expected.set_seconds(seconds);
  value.expected.set_nanos(nanos);
  value.value_to_test = str;
  return value;
}

class TestTimespanFromString : public testing::TestWithParam<struct timespan_test_case_from_string> {};

TEST_P(TestTimespanFromString, IsWellConverted) {
  auto value = GetParam();
  ::google::protobuf::Duration out;
  EXPECT_NO_THROW(out = armonik::api::common::utils::duration_from_timespan(value.value_to_test));
  EXPECT_EQ(out.seconds(), value.expected.seconds());
  EXPECT_EQ(out.nanos(), value.expected.nanos());
}

INSTANTIATE_TEST_SUITE_P(Timespan_Conversion, TestTimespanFromString,
                         ::testing::Values(genCaseForString(5, 0, "0:0:5"), genCaseForString(5, 5000, "0:0:5.000005"),
                                           genCaseForString(3605, 0, "1:0:5"), genCaseForString(-3605, 0, "-1:0:5"),
                                           genCaseForString(3605, 0, "1:0:5.0"), genCaseForString(3665, 0, "1:1:5.0"),
                                           genCaseForString(3605, 0, "0.1:0:5.0"),
                                           genCaseForString(90005, 0, "1.1:0:5"),
                                           genCaseForString(90005, 500000000, "1.1:0:5.5"),
                                           genCaseForString(90005, 500000000, "1.1:0:5.50000000000"),
                                           genCaseForString(-90005, -500000000, "-1.1:0:5.5")));
