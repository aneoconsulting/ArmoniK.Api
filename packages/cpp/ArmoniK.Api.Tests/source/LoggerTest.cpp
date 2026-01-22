#include <formatter.h>
#include <gtest/gtest.h>
#include <logger/logger.h>
#include <writer.h>

using namespace armonik::api::common::logger;

struct LogLevelParams {
  Level filter_level;
  Level call_level;

  LogLevelParams(std::tuple<Level, Level> params)
      : filter_level(std::get<0>(params)), call_level(std::get<1>(params)) {}
};

class LoggerTest : public testing::TestWithParam<std::tuple<Level, Level>> {};

TEST_P(LoggerTest, LogMessageIsFormattedAndWritten) {
  auto formatter_ptr = std::make_unique<MockFormatter>();
  auto writer_ptr = std::make_unique<MockWriter>();

  auto &formatter = *formatter_ptr;
  auto &writer = *writer_ptr;

  LogLevelParams params = GetParam();
  armonik::api::common::logger::Logger logger(std::move(writer_ptr), std::move(formatter_ptr), params.filter_level);

  if (params.call_level >= params.filter_level) {
    EXPECT_CALL(formatter,
                format(params.call_level, absl::string_view("Test message"), testing::_, testing::_, testing::_))
        .WillOnce(testing::Return("Formatted message"));

    EXPECT_CALL(writer, write(params.call_level, absl::string_view("Formatted message"))).Times(1);
  } else {
    EXPECT_CALL(formatter, format(testing::_, testing::_, testing::_, testing::_, testing::_)).Times(0);
    EXPECT_CALL(writer, write(testing::_, testing::_)).Times(0);
  }

  logger.log(params.call_level, "Test message");
}

INSTANTIATE_TEST_SUITE_P(LogLevels, LoggerTest,
                         testing::Combine(testing::Values(Level::Verbose, Level::Debug, Level::Info, Level::Warning,
                                                          Level::Error, Level::Fatal),
                                          testing::Values(Level::Verbose, Level::Debug, Level::Info, Level::Warning,
                                                          Level::Error, Level::Fatal)));
