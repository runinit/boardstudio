#pragma once
#include "rust/cxx.h"
#include <cstdint>

namespace bs_oracle {
struct Facts;
Facts inspect_file(rust::Str path);
}  // namespace bs_oracle
