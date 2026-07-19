#pragma once

/**
 * C++ include for Aeryon native DSP kernels.
 * The public surface is the C ABI in dsp.h — no C++ classes cross the boundary.
 */
#include "aeryon/dsp.h"

namespace aeryon::dsp {

/// Legacy no-op retained for smoke-test compatibility during migration.
inline void initialize() noexcept {}

}  // namespace aeryon::dsp
