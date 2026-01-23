#pragma once

#include "gmock/gmock.h"

#include "logger/writer.h"

class MockWriter : public armonik::api::common::logger::IWriter {
public:
  MOCK_METHOD(void, write, (armonik::api::common::logger::Level level, absl::string_view message), (override));
};
