#pragma once

#include <absl/strings/string_view.h>
#include <fmt/std.h>

const fmt::string_view to_fmt(const absl::string_view sv) { return fmt::string_view(sv.data(), sv.size()); }