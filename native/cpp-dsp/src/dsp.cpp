#include "aeryon/dsp.h"

#include <cmath>
#include <cstdint>
#include <limits>

namespace {

constexpr double kPi = 3.14159265358979323846;
constexpr double kTau = 2.0 * kPi;

[[nodiscard]] bool is_finite_f32(float value) noexcept {
    return std::isfinite(value) != 0;
}

[[nodiscard]] bool is_finite_f64(double value) noexcept {
    return std::isfinite(value) != 0;
}

[[nodiscard]] bool multiply_size(size_t a, size_t b, size_t* out) noexcept {
    if (a != 0 && b > (std::numeric_limits<size_t>::max() / a)) {
        return false;
    }
    *out = a * b;
    return true;
}

aeryon_dsp_status motion_energy_impl(
    const float* real_samples,
    const float* imag_samples,
    size_t frame_count,
    size_t subcarrier_count,
    double* output,
    size_t output_length
) {
    if (real_samples == nullptr || imag_samples == nullptr || output == nullptr) {
        return AERYON_DSP_NULL_POINTER;
    }
    if (frame_count < 2 || subcarrier_count == 0) {
        return AERYON_DSP_INVALID_LENGTH;
    }

    size_t sample_count = 0;
    if (!multiply_size(frame_count, subcarrier_count, &sample_count)) {
        return AERYON_DSP_INVALID_LENGTH;
    }

    const size_t required = frame_count - 1;
    if (output_length < required) {
        return AERYON_DSP_OUTPUT_TOO_SMALL;
    }

    for (size_t index = 0; index < sample_count; ++index) {
        if (!is_finite_f32(real_samples[index]) || !is_finite_f32(imag_samples[index])) {
            return AERYON_DSP_NON_FINITE_INPUT;
        }
    }

    const double inv_sc = 1.0 / static_cast<double>(subcarrier_count);
    for (size_t t = 1; t < frame_count; ++t) {
        double sum_sq = 0.0;
        const size_t prev_base = (t - 1) * subcarrier_count;
        const size_t curr_base = t * subcarrier_count;
        for (size_t k = 0; k < subcarrier_count; ++k) {
            const double dr = static_cast<double>(real_samples[curr_base + k])
                - static_cast<double>(real_samples[prev_base + k]);
            const double di = static_cast<double>(imag_samples[curr_base + k])
                - static_cast<double>(imag_samples[prev_base + k]);
            sum_sq += dr * dr + di * di;
        }
        const double energy = std::sqrt(sum_sq * inv_sc);
        if (!is_finite_f64(energy)) {
            return AERYON_DSP_NON_FINITE_INPUT;
        }
        output[t - 1] = energy;
    }
    return AERYON_DSP_OK;
}

aeryon_dsp_status center_hann_impl(
    const double* input,
    size_t input_length,
    double* output,
    size_t output_length
) {
    if (input == nullptr || output == nullptr) {
        return AERYON_DSP_NULL_POINTER;
    }
    if (input_length == 0) {
        return AERYON_DSP_INVALID_LENGTH;
    }
    if (output_length != input_length) {
        return AERYON_DSP_DIMENSION_MISMATCH;
    }

    for (size_t index = 0; index < input_length; ++index) {
        if (!is_finite_f64(input[index])) {
            return AERYON_DSP_NON_FINITE_INPUT;
        }
    }

    double sum = 0.0;
    for (size_t index = 0; index < input_length; ++index) {
        sum += input[index];
    }
    const double mean = sum / static_cast<double>(input_length);

    if (input_length == 1) {
        // Shared one-element policy: mean removal yields zero; Hann weight is 1.0.
        output[0] = 0.0;
        return AERYON_DSP_OK;
    }

    const double denom = static_cast<double>(input_length - 1);
    for (size_t n = 0; n < input_length; ++n) {
        const double phase = kTau * static_cast<double>(n) / denom;
        const double weight = 0.5 * (1.0 - std::cos(phase));
        const double centered = input[n] - mean;
        const double value = centered * weight;
        if (!is_finite_f64(value)) {
            return AERYON_DSP_INTERNAL_ERROR;
        }
        output[n] = value;
    }
    return AERYON_DSP_OK;
}

}  // namespace

extern "C" {

int32_t aeryon_dsp_abi_version(void) {
    return AERYON_DSP_ABI_VERSION;
}

aeryon_dsp_status aeryon_dsp_library_version(
    int32_t* major,
    int32_t* minor,
    int32_t* patch
) {
    try {
        if (major == nullptr || minor == nullptr || patch == nullptr) {
            return AERYON_DSP_NULL_POINTER;
        }
        *major = AERYON_DSP_VERSION_MAJOR;
        *minor = AERYON_DSP_VERSION_MINOR;
        *patch = AERYON_DSP_VERSION_PATCH;
        return AERYON_DSP_OK;
    } catch (...) {
        return AERYON_DSP_INTERNAL_ERROR;
    }
}

aeryon_dsp_status aeryon_dsp_motion_energy_f32(
    const float* real_samples,
    const float* imag_samples,
    size_t frame_count,
    size_t subcarrier_count,
    double* output,
    size_t output_length
) {
    try {
        return motion_energy_impl(
            real_samples,
            imag_samples,
            frame_count,
            subcarrier_count,
            output,
            output_length
        );
    } catch (...) {
        return AERYON_DSP_INTERNAL_ERROR;
    }
}

aeryon_dsp_status aeryon_dsp_center_hann_f64(
    const double* input,
    size_t input_length,
    double* output,
    size_t output_length
) {
    try {
        return center_hann_impl(input, input_length, output, output_length);
    } catch (...) {
        return AERYON_DSP_INTERNAL_ERROR;
    }
}

}  // extern "C"
