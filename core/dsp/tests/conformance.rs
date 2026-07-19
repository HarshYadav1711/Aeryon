//! Backend-independent numerical conformance for DSP kernels.
//!
//! Rust kernels always run. When the `cpp-dsp` feature is enabled, outputs are
//! compared against the C++ backend within documented floating-point tolerances.

use aeryon_dsp::{
    DspBackendKind, DspError, DspErrorCode, DspKernelBackend, MotionEnergyInput, RustKernelBackend,
    create_backend, kernels,
};

const ABS_TOL: f64 = 1e-5;
const REL_TOL: f64 = 1e-4;

fn within_tolerance(expected: f64, actual: f64) -> bool {
    let err = (expected - actual).abs();
    if err <= ABS_TOL {
        return true;
    }
    let scale = expected.abs().max(actual.abs()).max(f64::MIN_POSITIVE);
    err <= REL_TOL * scale
}

fn assert_slices_close(label: &str, expected: &[f64], actual: &[f64], backend: &str) {
    assert_eq!(
        expected.len(),
        actual.len(),
        "{label}: length mismatch (backend={backend}, expected_len={}, actual_len={})",
        expected.len(),
        actual.len()
    );
    for (index, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
        if !within_tolerance(*exp, *act) {
            let abs_err = (exp - act).abs();
            let scale = exp.abs().max(act.abs()).max(f64::MIN_POSITIVE);
            let rel_err = abs_err / scale;
            panic!(
                "{label}: mismatch at index {index} (backend={backend})\n\
                 expected={exp}\n\
                 actual={act}\n\
                 abs_err={abs_err}\n\
                 rel_err={rel_err}\n\
                 abs_tol={ABS_TOL} rel_tol={REL_TOL}"
            );
        }
    }
}

fn motion(
    backend: &dyn DspKernelBackend,
    real: &[f32],
    imag: &[f32],
    frames: usize,
    sc: usize,
) -> Result<Vec<f64>, DspError> {
    backend.motion_energy(MotionEnergyInput {
        real_samples: real,
        imag_samples: imag,
        frame_count: frames,
        subcarrier_count: sc,
    })
}

fn deterministic_csi(frames: usize, sc: usize, mag_scale: f32) -> (Vec<f32>, Vec<f32>) {
    let n = frames * sc;
    let mut real = Vec::with_capacity(n);
    let mut imag = Vec::with_capacity(n);
    for t in 0..frames {
        for k in 0..sc {
            let phase = (t as f32) * 0.37 + (k as f32) * 0.11;
            let mag = mag_scale * (1.0 + 0.05 * (k as f32));
            // Mixed magnitudes and signs — deterministic, no RNG dependency.
            let sign = if (t + k) % 3 == 0 { -1.0_f32 } else { 1.0 };
            real.push(sign * mag * phase.cos());
            imag.push(sign * 0.5 * mag * phase.sin());
        }
    }
    (real, imag)
}

fn deterministic_signal(len: usize, offset: f64, amplitude: f64) -> Vec<f64> {
    (0..len)
        .map(|n| {
            let x = n as f64;
            offset + amplitude * (0.4 * x).sin() + 0.05 * (0.17 * x).cos()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rust-only thorough coverage (always compiled)
// ---------------------------------------------------------------------------

#[test]
fn rust_motion_identical_frames_is_zero() {
    let real = [1.0_f32, 0.5, 1.0, 0.5];
    let imag = [0.25_f32, -0.1, 0.25, -0.1];
    let out = kernels::motion_energy_link(&real, &imag, 2, 2).expect("ok");
    assert_eq!(out.len(), 1);
    assert!(
        out[0].abs() < ABS_TOL,
        "identical frames → ~0, got {}",
        out[0]
    );
}

#[test]
fn rust_motion_known_real_difference() {
    // (1,0) → (2,0) over one SC: energy = |1| = 1
    let out = kernels::motion_energy_link(&[1.0, 2.0], &[0.0, 0.0], 2, 1).expect("ok");
    assert!((out[0] - 1.0).abs() < ABS_TOL);
}

#[test]
fn rust_motion_known_imaginary_difference() {
    // (0,0) → (0,1): energy = 1
    let out = kernels::motion_energy_link(&[0.0, 0.0], &[0.0, 1.0], 2, 1).expect("ok");
    assert!((out[0] - 1.0).abs() < ABS_TOL);
}

#[test]
fn rust_motion_sqrt2_reference() {
    let out = kernels::motion_energy_link(&[1.0, 0.0], &[0.0, 1.0], 2, 1).expect("ok");
    assert!((out[0] - std::f64::consts::SQRT_2).abs() < ABS_TOL);
}

#[test]
fn rust_motion_multiple_subcarriers_and_transitions() {
    let frames = 4;
    let sc = 3;
    let (real, imag) = deterministic_csi(frames, sc, 2.5);
    let out = kernels::motion_energy_link(&real, &imag, frames, sc).expect("ok");
    assert_eq!(out.len(), frames - 1);
    assert!(out.iter().all(|v| v.is_finite() && *v >= 0.0));
}

#[test]
fn rust_motion_negative_and_near_zero() {
    let real = [-1e-8_f32, 1e-8, -2e-8];
    let imag = [1e-8_f32, -1e-8, 0.0];
    let out = kernels::motion_energy_link(&real, &imag, 3, 1).expect("ok");
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|v| v.is_finite() && *v >= 0.0));
}

#[test]
fn rust_motion_rejects_non_finite() {
    let err = kernels::motion_energy_link(&[1.0, f32::NAN], &[0.0, 0.0], 2, 1).expect_err("nan");
    assert!(matches!(err, DspError::MotionEnergy { .. }));
}

#[test]
fn rust_motion_rejects_malformed_dimensions() {
    let err = kernels::motion_energy_link(&[1.0], &[0.0, 1.0], 2, 1).expect_err("mismatch");
    assert!(matches!(err, DspError::MotionEnergy { .. }));
    let err = kernels::motion_energy_link(&[1.0], &[0.0], 1, 1).expect_err("frames");
    assert!(matches!(err, DspError::MotionEnergy { .. }));
    let err = kernels::motion_energy_link(&[], &[], 2, 0).expect_err("sc");
    assert!(matches!(err, DspError::MotionEnergy { .. }));
}

#[test]
fn rust_center_hann_constant_is_zero() {
    let out = kernels::center_and_apply_hann(&[2.5; 16]).expect("ok");
    assert!(out.iter().all(|v| v.abs() < ABS_TOL));
}

#[test]
fn rust_center_hann_one_element_policy() {
    let out = kernels::center_and_apply_hann(&[9.0]).expect("ok");
    assert_eq!(out, vec![0.0]);
}

#[test]
fn rust_center_hann_sine_and_offset() {
    let sine = deterministic_signal(32, 0.0, 1.0);
    let offset = deterministic_signal(32, 3.5, 1.0);
    let a = kernels::center_and_apply_hann(&sine).expect("sine");
    let b = kernels::center_and_apply_hann(&offset).expect("offset");
    assert_eq!(a.len(), 32);
    assert_eq!(b.len(), 32);
    // Mean removal ⇒ DC-heavy offset sine should closely match the zero-mean case.
    assert_slices_close("offset-vs-zero-mean", &a, &b, "rust");
}

#[test]
fn rust_center_hann_negative_values() {
    let out = kernels::center_and_apply_hann(&[-2.0, -1.0, 0.0, 1.0, 2.0]).expect("ok");
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(out[0].abs() < ABS_TOL && out[4].abs() < ABS_TOL); // Hann ends at 0
}

#[test]
fn rust_center_hann_rejects_empty_and_non_finite() {
    let err = kernels::center_and_apply_hann(&[]).expect_err("empty");
    assert_eq!(err.code(), DspErrorCode::InsufficientLength);
    let err = kernels::center_and_apply_hann(&[1.0, f64::INFINITY]).expect_err("inf");
    assert_eq!(err.code(), DspErrorCode::NonFinite);
}

#[test]
fn rust_property_style_motion_dimensions() {
    let backend = RustKernelBackend;
    for &frames in &[2usize, 4, 8, 16, 32, 64] {
        for &sc in &[1usize, 4, 16, 32, 64] {
            for &mag in &[1.0_f32, 1e3, 1e-3] {
                let (real, imag) = deterministic_csi(frames, sc, mag);
                let out = motion(&backend, &real, &imag, frames, sc)
                    .unwrap_or_else(|e| panic!("rust motion frames={frames} sc={sc}: {e}"));
                assert_eq!(out.len(), frames - 1);
                assert!(out.iter().all(|v| v.is_finite() && *v >= 0.0));
            }
        }
    }
}

#[test]
fn rust_property_style_center_hann_lengths() {
    let backend = RustKernelBackend;
    for &len in &[1usize, 4, 8, 16, 32, 64] {
        let signal = if len == 1 {
            vec![7.0]
        } else {
            deterministic_signal(len, -0.5, 2.0)
        };
        let out = backend
            .center_and_apply_hann(&signal)
            .unwrap_or_else(|e| panic!("rust center_hann len={len}: {e}"));
        assert_eq!(out.len(), len);
        assert!(out.iter().all(|v| v.is_finite()));
    }
}

#[test]
#[cfg(not(feature = "cpp-dsp"))]
fn cpp_parity_requires_cpp_dsp_feature() {
    // Documented skip: Rust↔C++ numerical comparison runs only with `--features cpp-dsp`.
    let err = match create_backend(DspBackendKind::Cpp) {
        Err(error) => error,
        Ok(_) => panic!("cpp unavailable without feature"),
    };
    match err {
        DspError::BackendUnavailable {
            backend: DspBackendKind::Cpp,
            ..
        } => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Rust ↔ C++ parity (cpp-dsp feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "cpp-dsp")]
mod cpp_parity {
    use super::*;

    fn cpp_backend() -> std::sync::Arc<dyn DspKernelBackend> {
        create_backend(DspBackendKind::Cpp).expect("cpp backend")
    }

    #[test]
    fn motion_milestone_cases_match() {
        let rust = RustKernelBackend;
        let cpp = cpp_backend();

        // Identical frames
        let real = [1.0_f32, 0.5, 1.0, 0.5];
        let imag = [0.25_f32, -0.1, 0.25, -0.1];
        let r = motion(&rust, &real, &imag, 2, 2).expect("rust");
        let c = motion(cpp.as_ref(), &real, &imag, 2, 2).expect("cpp");
        assert_slices_close("identical", &r, &c, "cpp");

        // Known real / imag / sqrt2
        for (label, re, im) in [
            (
                "real-diff",
                [1.0_f32, 2.0].as_slice(),
                [0.0_f32, 0.0].as_slice(),
            ),
            (
                "imag-diff",
                [0.0_f32, 0.0].as_slice(),
                [0.0_f32, 1.0].as_slice(),
            ),
            (
                "sqrt2",
                [1.0_f32, 0.0].as_slice(),
                [0.0_f32, 1.0].as_slice(),
            ),
        ] {
            let r = motion(&rust, re, im, 2, 1).expect("rust");
            let c = motion(cpp.as_ref(), re, im, 2, 1).expect("cpp");
            assert_slices_close(label, &r, &c, "cpp");
        }

        // Multi-SC / multi-transition / negatives / near-zero
        let (real, imag) = deterministic_csi(8, 4, 1.5);
        let r = motion(&rust, &real, &imag, 8, 4).expect("rust");
        let c = motion(cpp.as_ref(), &real, &imag, 8, 4).expect("cpp");
        assert_slices_close("multi", &r, &c, "cpp");

        let real = [-1e-8_f32, 1e-8, -2e-8];
        let imag = [1e-8_f32, -1e-8, 0.0];
        let r = motion(&rust, &real, &imag, 3, 1).expect("rust");
        let c = motion(cpp.as_ref(), &real, &imag, 3, 1).expect("cpp");
        assert_slices_close("near-zero", &r, &c, "cpp");
    }

    #[test]
    fn center_hann_milestone_cases_match() {
        let rust = RustKernelBackend;
        let cpp = cpp_backend();

        for signal in [
            vec![3.0; 16],
            deterministic_signal(32, 0.0, 1.0),
            deterministic_signal(32, 4.0, 1.0),
            vec![-2.0, -1.0, 0.0, 1.0, 2.0],
            vec![42.0],
        ] {
            let r = rust.center_and_apply_hann(&signal).expect("rust");
            let c = cpp.center_and_apply_hann(&signal).expect("cpp");
            assert_slices_close("center_hann", &r, &c, "cpp");
        }
    }

    #[test]
    fn property_style_motion_parity() {
        let rust = RustKernelBackend;
        let cpp = cpp_backend();
        for &frames in &[2usize, 4, 8, 16, 32, 64] {
            for &sc in &[1usize, 4, 16, 32, 64] {
                for &mag in &[1.0_f32, 1e3, 1e-3] {
                    let (real, imag) = deterministic_csi(frames, sc, mag);
                    let r = motion(&rust, &real, &imag, frames, sc)
                        .unwrap_or_else(|e| panic!("rust frames={frames} sc={sc}: {e}"));
                    let c = motion(cpp.as_ref(), &real, &imag, frames, sc)
                        .unwrap_or_else(|e| panic!("cpp frames={frames} sc={sc}: {e}"));
                    assert_slices_close(
                        &format!("property motion frames={frames} sc={sc} mag={mag}"),
                        &r,
                        &c,
                        "cpp",
                    );
                }
            }
        }
    }

    #[test]
    fn property_style_center_hann_parity() {
        let rust = RustKernelBackend;
        let cpp = cpp_backend();
        for &len in &[1usize, 4, 8, 16, 32, 64] {
            let signal = if len == 1 {
                vec![7.0]
            } else {
                deterministic_signal(len, -0.5, 2.0)
            };
            let r = rust
                .center_and_apply_hann(&signal)
                .unwrap_or_else(|e| panic!("rust len={len}: {e}"));
            let c = cpp
                .center_and_apply_hann(&signal)
                .unwrap_or_else(|e| panic!("cpp len={len}: {e}"));
            assert_slices_close(&format!("property center_hann len={len}"), &r, &c, "cpp");
        }
    }

    #[test]
    fn rejection_parity_non_finite_and_dimensions() {
        let rust = RustKernelBackend;
        let cpp = cpp_backend();

        assert!(motion(&rust, &[1.0, f32::NAN], &[0.0, 0.0], 2, 1).is_err());
        assert!(motion(cpp.as_ref(), &[1.0, f32::NAN], &[0.0, 0.0], 2, 1).is_err());

        assert!(motion(&rust, &[1.0], &[0.0, 1.0], 2, 1).is_err());
        assert!(motion(cpp.as_ref(), &[1.0], &[0.0, 1.0], 2, 1).is_err());

        assert!(rust.center_and_apply_hann(&[]).is_err());
        assert!(cpp.center_and_apply_hann(&[]).is_err());
        assert!(rust.center_and_apply_hann(&[f64::NAN]).is_err());
        assert!(cpp.center_and_apply_hann(&[f64::NAN]).is_err());
    }
}
