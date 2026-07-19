#include "aeryon/dsp.h"
#include "aeryon/dsp.hpp"

#include <cmath>
#include <cstdlib>
#include <iostream>
#include <limits>
#include <vector>

namespace {

int failures = 0;

void expect(bool condition, const char* message) {
    if (!condition) {
        std::cerr << "FAIL: " << message << '\n';
        ++failures;
    }
}

void expect_near(double actual, double expected, double tol, const char* message) {
    if (!(std::isfinite(actual) && std::abs(actual - expected) <= tol)) {
        std::cerr << "FAIL: " << message << " expected=" << expected << " actual=" << actual
                  << '\n';
        ++failures;
    }
}

void test_abi_and_version() {
    expect(aeryon_dsp_abi_version() == AERYON_DSP_ABI_VERSION, "abi version");
    int32_t major = -1;
    int32_t minor = -1;
    int32_t patch = -1;
    expect(
        aeryon_dsp_library_version(&major, &minor, &patch) == AERYON_DSP_OK,
        "library version ok"
    );
    expect(major == AERYON_DSP_VERSION_MAJOR, "major");
    expect(minor == AERYON_DSP_VERSION_MINOR, "minor");
    expect(patch == AERYON_DSP_VERSION_PATCH, "patch");
    expect(
        aeryon_dsp_library_version(nullptr, &minor, &patch) == AERYON_DSP_NULL_POINTER,
        "version null"
    );
}

void test_motion_energy_correctness() {
    // Two frames, one subcarrier: (1,0) -> (0,1) => energy = sqrt(1+1) = sqrt(2)
    const float real_s[] = {1.0f, 0.0f};
    const float imag_s[] = {0.0f, 1.0f};
    double out[1] = {0.0};
    expect(
        aeryon_dsp_motion_energy_f32(real_s, imag_s, 2, 1, out, 1) == AERYON_DSP_OK,
        "motion ok"
    );
    expect_near(out[0], std::sqrt(2.0), 1e-12, "motion sqrt2");

    // Identical frames → ~0
    const float real_z[] = {1.0f, 0.5f, 1.0f, 0.5f};
    const float imag_z[] = {0.25f, -0.1f, 0.25f, -0.1f};
    double out_z[1] = {1.0};
    expect(
        aeryon_dsp_motion_energy_f32(real_z, imag_z, 2, 2, out_z, 1) == AERYON_DSP_OK,
        "identical motion ok"
    );
    expect_near(out_z[0], 0.0, 1e-12, "identical motion zero");
}

void test_center_hann() {
    // Constant → zeros after mean removal
    const double input[] = {3.0, 3.0, 3.0, 3.0};
    double out[4] = {1.0, 1.0, 1.0, 1.0};
    expect(aeryon_dsp_center_hann_f64(input, 4, out, 4) == AERYON_DSP_OK, "center ok");
    for (double value : out) {
        expect_near(value, 0.0, 1e-15, "constant centered hann");
    }

    // Hann coefficients for N=4: 0, 0.5, 1.0? Wait: 0.5*(1-cos(2pi n / 3))
    // n=0 → 0, n=1 → 0.5*(1-cos(2pi/3))=0.5*(1-(-0.5))=0.75, n=2 → 0.75, n=3 → 0
    const double ramp[] = {0.0, 1.0, 2.0, 3.0};
    double out_r[4] = {};
    expect(aeryon_dsp_center_hann_f64(ramp, 4, out_r, 4) == AERYON_DSP_OK, "ramp ok");
    const double mean = 1.5;
    const double w0 = 0.0;
    const double w1 = 0.5 * (1.0 - std::cos(2.0 * 3.14159265358979323846 / 3.0));
    const double w2 = 0.5 * (1.0 - std::cos(4.0 * 3.14159265358979323846 / 3.0));
    const double w3 = 0.0;
    expect_near(out_r[0], (0.0 - mean) * w0, 1e-12, "hann0");
    expect_near(out_r[1], (1.0 - mean) * w1, 1e-12, "hann1");
    expect_near(out_r[2], (2.0 - mean) * w2, 1e-12, "hann2");
    expect_near(out_r[3], (3.0 - mean) * w3, 1e-12, "hann3");

    // One-element policy
    const double one[] = {42.0};
    double out_one[1] = {1.0};
    expect(aeryon_dsp_center_hann_f64(one, 1, out_one, 1) == AERYON_DSP_OK, "one ok");
    expect_near(out_one[0], 0.0, 0.0, "one element zero");
}

void test_rejections() {
    float real_s[] = {1.0f, 0.0f};
    float imag_s[] = {0.0f, 1.0f};
    double out[1] = {};

    expect(
        aeryon_dsp_motion_energy_f32(nullptr, imag_s, 2, 1, out, 1) == AERYON_DSP_NULL_POINTER,
        "null real"
    );
    expect(
        aeryon_dsp_motion_energy_f32(real_s, imag_s, 1, 1, out, 1) == AERYON_DSP_INVALID_LENGTH,
        "one frame"
    );
    expect(
        aeryon_dsp_motion_energy_f32(real_s, imag_s, 2, 0, out, 1) == AERYON_DSP_INVALID_LENGTH,
        "zero sc"
    );
    expect(
        aeryon_dsp_motion_energy_f32(real_s, imag_s, 2, 1, out, 0) == AERYON_DSP_OUTPUT_TOO_SMALL,
        "output small"
    );

    float nan_real[] = {1.0f, std::numeric_limits<float>::quiet_NaN()};
    float nan_imag[] = {0.0f, 0.0f};
    expect(
        aeryon_dsp_motion_energy_f32(nan_real, nan_imag, 2, 1, out, 1)
            == AERYON_DSP_NON_FINITE_INPUT,
        "nan motion"
    );

    const double empty_in[] = {1.0};
    double empty_out[1] = {};
    expect(
        aeryon_dsp_center_hann_f64(nullptr, 1, empty_out, 1) == AERYON_DSP_NULL_POINTER,
        "null center"
    );
    expect(
        aeryon_dsp_center_hann_f64(empty_in, 0, empty_out, 0) == AERYON_DSP_INVALID_LENGTH,
        "empty center"
    );
    expect(
        aeryon_dsp_center_hann_f64(empty_in, 1, empty_out, 2) == AERYON_DSP_DIMENSION_MISMATCH,
        "len mismatch"
    );
    const double inf_in[] = {std::numeric_limits<double>::infinity()};
    expect(
        aeryon_dsp_center_hann_f64(inf_in, 1, empty_out, 1) == AERYON_DSP_NON_FINITE_INPUT,
        "inf center"
    );
}

}  // namespace

int main() {
    aeryon::dsp::initialize();
    test_abi_and_version();
    test_motion_energy_correctness();
    test_center_hann();
    test_rejections();

    if (failures != 0) {
        std::cerr << failures << " assertion(s) failed\n";
        return EXIT_FAILURE;
    }
    std::cout << "all native DSP tests passed\n";
    return EXIT_SUCCESS;
}
