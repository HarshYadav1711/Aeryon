//! Backend selection and native failure mapping — no silent Rust fallback.

use aeryon_dsp::{DspBackendKind, DspConfig, DspError, DspErrorCode, create_backend};

#[cfg(feature = "cpp-dsp")]
use aeryon_dsp::MotionEnergyInput;

#[test]
fn config_rejects_cpp_when_not_compiled() {
    let config = DspConfig {
        enabled: true,
        backend: DspBackendKind::Cpp,
        ..DspConfig::default()
    };

    if DspBackendKind::Cpp.is_compiled() {
        // With cpp-dsp, validation should succeed (backend is compiled).
        config
            .validate()
            .expect("cpp available when feature enabled");
        return;
    }

    let err = match config.validate() {
        Err(error) => error,
        Ok(()) => panic!("cpp must be rejected without feature"),
    };
    match err {
        DspError::BackendUnavailable {
            backend: DspBackendKind::Cpp,
            message,
        } => {
            assert!(
                message.contains("cpp-dsp") || message.contains("not available"),
                "message should mention feature/build: {message}"
            );
        }
        other => panic!("expected BackendUnavailable, got {other:?}"),
    }
}

#[test]
fn create_backend_does_not_silently_fall_back_to_rust() {
    if DspBackendKind::Cpp.is_compiled() {
        let backend = create_backend(DspBackendKind::Cpp).expect("cpp when compiled");
        assert_eq!(backend.identity().kind, DspBackendKind::Cpp);
        assert_ne!(backend.identity().kind, DspBackendKind::Rust);
        return;
    }

    let err = match create_backend(DspBackendKind::Cpp) {
        Err(error) => error,
        Ok(_) => panic!("must fail without feature"),
    };
    match err {
        DspError::BackendUnavailable {
            backend: DspBackendKind::Cpp,
            ..
        } => {}
        other => panic!("expected BackendUnavailable, got {other:?}"),
    }
    // Confirm Rust is still obtainable explicitly — failure was not a silent swap.
    let rust = create_backend(DspBackendKind::Rust).expect("rust");
    assert_eq!(rust.identity().kind, DspBackendKind::Rust);
}

#[test]
fn rust_backend_never_reports_cpp_identity() {
    let backend = create_backend(DspBackendKind::Rust).expect("rust");
    let identity = backend.identity();
    assert_eq!(identity.kind, DspBackendKind::Rust);
    assert_eq!(identity.kind.as_str(), "rust");
    assert!(identity.abi_version.is_none());
    assert!(identity.build_available);
}

#[test]
fn disabled_config_skips_backend_availability_check() {
    let config = DspConfig {
        enabled: false,
        backend: DspBackendKind::Cpp,
        ..DspConfig::default()
    };
    // Disabled DSP must not fail startup solely due to backend selection.
    config.validate().expect("disabled config ok");
}

#[cfg(feature = "cpp-dsp")]
mod with_native {
    use super::*;

    #[test]
    fn cpp_backend_initializes_with_abi() {
        let backend = create_backend(DspBackendKind::Cpp).expect("cpp");
        let identity = backend.identity();
        assert_eq!(identity.kind, DspBackendKind::Cpp);
        assert!(identity.abi_version.is_some());
        assert_eq!(identity.abi_version, Some(aeryon_dsp::CPP_ABI_VERSION));
        assert!(identity.build_available);
    }

    #[test]
    fn native_motion_status_maps_dimension_mismatch() {
        let backend = create_backend(DspBackendKind::Cpp).expect("cpp");
        let err = match backend.motion_energy(MotionEnergyInput {
            real_samples: &[1.0],
            imag_samples: &[0.0, 1.0],
            frame_count: 2,
            subcarrier_count: 1,
        }) {
            Err(error) => error,
            Ok(_) => panic!("dimension mismatch"),
        };

        assert_eq!(err.code(), DspErrorCode::NativeKernel);
        match &err {
            DspError::NativeKernel {
                backend: DspBackendKind::Cpp,
                kernel: "motion_energy",
                status,
                message,
            } => {
                assert_eq!(status, "dimension_mismatch");
                assert!(
                    message.contains("dims=") || message.contains("dimension"),
                    "message should carry dimensions: {message}"
                );
            }
            // Pre-validated in the FFI layer before the native call — still not a silent OK.
            DspError::NativeKernel { .. } => {}
            other => panic!("expected NativeKernel mapping, got {other:?}"),
        }
    }

    #[test]
    fn native_motion_status_maps_invalid_length() {
        let backend = create_backend(DspBackendKind::Cpp).expect("cpp");
        let err = match backend.motion_energy(MotionEnergyInput {
            real_samples: &[1.0],
            imag_samples: &[0.0],
            frame_count: 1,
            subcarrier_count: 1,
        }) {
            Err(error) => error,
            Ok(_) => panic!("invalid length"),
        };

        match &err {
            DspError::NativeKernel {
                backend: DspBackendKind::Cpp,
                kernel: "motion_energy",
                status,
                ..
            } => {
                assert!(
                    status == "invalid_length" || status == "dimension_mismatch",
                    "unexpected status label {status}"
                );
            }
            other => panic!("expected NativeKernel, got {other:?}"),
        }
    }

    #[test]
    fn native_center_hann_status_maps_empty_input() {
        let backend = create_backend(DspBackendKind::Cpp).expect("cpp");
        let err = match backend.center_and_apply_hann(&[]) {
            Err(error) => error,
            Ok(_) => panic!("empty input"),
        };

        match err {
            DspError::NativeKernel {
                backend: DspBackendKind::Cpp,
                kernel: "center_hann",
                status,
                ..
            } => {
                assert_eq!(status, "invalid_length");
            }
            other => panic!("expected NativeKernel, got {other:?}"),
        }
    }

    #[test]
    fn native_center_hann_status_maps_non_finite() {
        let backend = create_backend(DspBackendKind::Cpp).expect("cpp");
        let err = match backend.center_and_apply_hann(&[1.0, f64::NAN]) {
            Err(error) => error,
            Ok(_) => panic!("non-finite"),
        };

        match err {
            DspError::NativeKernel {
                backend: DspBackendKind::Cpp,
                kernel: "center_hann",
                status,
                ..
            } => {
                assert_eq!(status, "non_finite_input");
            }
            other => panic!("expected NativeKernel, got {other:?}"),
        }
    }

    #[test]
    fn config_accepts_cpp_when_feature_enabled() {
        let config = DspConfig {
            enabled: true,
            backend: DspBackendKind::Cpp,
            ..DspConfig::default()
        };
        config.validate().expect("cpp config valid with feature");
        let profile = config.resolve_profile().expect("profile");
        assert_eq!(profile.backend, DspBackendKind::Cpp);
    }
}

#[cfg(not(feature = "cpp-dsp"))]
mod without_native {
    use super::*;

    #[test]
    fn is_compiled_reports_cpp_unavailable() {
        assert!(!DspBackendKind::Cpp.is_compiled());
        assert!(DspBackendKind::Rust.is_compiled());
    }

    #[test]
    fn enabled_cpp_config_surfaces_backend_unavailable_code() {
        let config = DspConfig {
            enabled: true,
            backend: DspBackendKind::Cpp,
            ..DspConfig::default()
        };
        let err = match config.validate() {
            Err(error) => error,
            Ok(()) => panic!("unavailable"),
        };
        assert_eq!(err.code(), DspErrorCode::BackendUnavailable);
    }
}
