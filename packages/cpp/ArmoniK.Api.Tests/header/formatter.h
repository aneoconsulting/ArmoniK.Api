#pragma once

#include "gmock/gmock.h"

#include "logger/formatter.h"

class MockFormatter : public armonik::api::common::logger::IFormatter {
public:
  MOCK_METHOD(std::string, format,
              (armonik::api::common::logger::Level level, absl::string_view message,
               const armonik::api::common::logger::Context &global_context,
               const armonik::api::common::logger::Context &local_context,
               const armonik::api::common::logger::Context &message_context),
              (override));
};
