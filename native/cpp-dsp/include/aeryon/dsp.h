/**
 * @file dsp.h
 * @brief Minimal C ABI for Aeryon numerical DSP kernels.
 *
 * Design rules:
 * - Primitive types and explicit lengths only.
 * - Caller-owned output buffers (Rust owns allocations).
 * - No C++ types, STL containers, exceptions, or ownership transfer.
 * - Deterministic; no global mutable state.
 *
 * Motion-energy input layout (one antenna link):
 *   Flattened [frame][subcarrier] with separate contiguous real/imag f32 arrays.
 *   Index of (frame t, subcarrier k) = t * subcarrier_count + k.
 *
 * Motion-energy formula (matches Rust reference):
 *   energy[t] = sqrt(mean_k( (re[t,k]-re[t-1,k])^2 + (im[t,k]-im[t-1,k])^2 ))
 *   Output length = frame_count - 1. Units: calibrated complex-difference energy proxy.
 *
 * Center + Hann:
 *   1) subtract arithmetic mean
 *   2) apply symmetric Hann: w[n] = 0.5 * (1 - cos(2*pi*n/(N-1))) for N > 1
 *      N == 1 → weight 1.0 (single-element policy shared with Rust)
 *   Empty input is rejected. Output length equals input length.
 */

#ifndef AERYON_DSP_H
#define AERYON_DSP_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Stable ABI version advertised by this header and library. */
#define AERYON_DSP_ABI_VERSION 1

/** Library version components (semver-style). */
#define AERYON_DSP_VERSION_MAJOR 0
#define AERYON_DSP_VERSION_MINOR 1
#define AERYON_DSP_VERSION_PATCH 0

/**
 * Stable native status codes.
 *
 * Meanings are fixed for ABI compatibility; do not renumber.
 */
typedef enum aeryon_dsp_status {
    AERYON_DSP_OK = 0,
    AERYON_DSP_NULL_POINTER = 1,
    AERYON_DSP_INVALID_LENGTH = 2,
    AERYON_DSP_DIMENSION_MISMATCH = 3,
    AERYON_DSP_NON_FINITE_INPUT = 4,
    AERYON_DSP_OUTPUT_TOO_SMALL = 5,
    AERYON_DSP_UNSUPPORTED_ABI = 6,
    AERYON_DSP_INTERNAL_ERROR = 7
} aeryon_dsp_status;

/** Returns the compiled ABI version (must equal AERYON_DSP_ABI_VERSION). */
int32_t aeryon_dsp_abi_version(void);

/**
 * Writes library version components into caller-owned integers.
 * Any null pointer is rejected.
 */
aeryon_dsp_status aeryon_dsp_library_version(
    int32_t* major,
    int32_t* minor,
    int32_t* patch
);

/**
 * Per-link temporal motion-energy kernel.
 *
 * @param real_samples      Contiguous f32 real parts, length frame_count * subcarrier_count
 * @param imag_samples      Contiguous f32 imag parts, same length
 * @param frame_count       Number of frames (>= 2)
 * @param subcarrier_count  Number of subcarriers (>= 1)
 * @param output            Caller-owned f64 buffer for energies
 * @param output_length     Must be >= frame_count - 1
 */
aeryon_dsp_status aeryon_dsp_motion_energy_f32(
    const float* real_samples,
    const float* imag_samples,
    size_t frame_count,
    size_t subcarrier_count,
    double* output,
    size_t output_length
);

/**
 * Temporal mean removal followed by symmetric Hann window.
 *
 * @param input         Contiguous f64 samples
 * @param input_length  Sample count (>= 1)
 * @param output        Caller-owned f64 buffer
 * @param output_length Must equal input_length
 */
aeryon_dsp_status aeryon_dsp_center_hann_f64(
    const double* input,
    size_t input_length,
    double* output,
    size_t output_length
);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* AERYON_DSP_H */
