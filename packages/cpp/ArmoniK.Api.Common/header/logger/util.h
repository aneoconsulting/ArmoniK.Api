#pragma once

#include <fmt/std.h>
#include <utils/string_view.h>

inline fmt::string_view to_fmt(const armonik::api::string_view sv) { return {sv.data(), sv.size()}; }
