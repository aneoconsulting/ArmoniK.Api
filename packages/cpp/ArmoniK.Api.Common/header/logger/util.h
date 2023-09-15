#pragma once

#include <absl/strings/string_view.h>
#include <fmt/std.h>

fmt::string_view to_fmt(const absl::string_view sv) { return {sv.data(), sv.size()}; }
