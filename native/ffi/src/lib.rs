//! Safe Rust wrappers around the Aeryon native C++ DSP C ABI.
//!
//! # Safety boundary
//!
//! All `unsafe` Rust is confined to this crate. Callers interact only with
//! validated, allocation-owning safe APIs. Raw pointers never escape these
//! wrappers, and C++ does not retain Rust pointers.

#![deny(missing_docs)]
#![allow(unsafe_code)]

use std::os::raw::{c_int, c_void};

use thiserror::Error;

/// Compiled native ABI version expected by this crate.
pub const EXPECTED_ABI_VERSION: i32 = 1;

/// Native library semver components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeLibraryVersion {
    /// Major version.
    pub major: i32,
    /// Minor version.
    pub minor: i32,
    /// Patch version.
    pub patch: i32,
}

impl NativeLibraryVersion {
    /// Human-readable `major.minor.patch`.
    pub fn display(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Stable native status codes mirrored from `aeryon/dsp.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NativeStatus {
    /// Success.
    Ok = 0,
    /// Required pointer was null.
    NullPointer = 1,
    /// Length argument was invalid.
    InvalidLength = 2,
    /// Input/output dimensions disagree.
    DimensionMismatch = 3,
    /// Non-finite input detected.
    NonFiniteInput = 4,
    /// Caller output buffer is too small.
    OutputTooSmall = 5,
    /// ABI version is unsupported.
    UnsupportedAbi = 6,
    /// Unexpected internal native failure.
    InternalError = 7,
}

impl NativeStatus {
    /// Converts a raw integer status, mapping unknowns to internal error.
    pub fn from_raw(value: i32) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::NullPointer,
            2 => Self::InvalidLength,
            3 => Self::DimensionMismatch,
            4 => Self::NonFiniteInput,
            5 => Self::OutputTooSmall,
            6 => Self::UnsupportedAbi,
            _ => Self::InternalError,
        }
    }

    /// Stable label for diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NullPointer => "null_pointer",
            Self::InvalidLength => "invalid_length",
            Self::DimensionMismatch => "dimension_mismatch",
            Self::NonFiniteInput => "non_finite_input",
            Self::OutputTooSmall => "output_too_small",
            Self::UnsupportedAbi => "unsupported_abi",
            Self::InternalError => "internal_error",
        }
    }
}

/// Errors from the native FFI boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FfiError {
    /// Native kernel returned a non-success status.
    #[error("native DSP {kernel} failed: status={status_label}; {context}")]
    Native {
        /// Kernel name (`motion_energy` or `center_hann`).
        kernel: &'static str,
        /// Raw status.
        status: NativeStatus,
        /// Status label.
        status_label: &'static str,
        /// Operator-safe context (dimensions, etc.).
        context: String,
    },
    /// Compiled ABI does not match this wrapper.
    #[error("native DSP ABI mismatch: expected {expected}, got {actual}")]
    AbiMismatch {
        /// Expected ABI version.
        expected: i32,
        /// Actual ABI version.
        actual: i32,
    },
}

mod bindings {
    use super::{c_int, c_void};

    pub type AeryonDspStatus = i32;

    unsafe extern "C" {
        pub fn aeryon_dsp_abi_version() -> i32;
        pub fn aeryon_dsp_library_version(
            major: *mut i32,
            minor: *mut i32,
            patch: *mut i32,
        ) -> AeryonDspStatus;
        pub fn aeryon_dsp_motion_energy_f32(
            real_samples: *const f32,
            imag_samples: *const f32,
            frame_count: usize,
            subcarrier_count: usize,
            output: *mut f64,
            output_length: usize,
        ) -> AeryonDspStatus;
        pub fn aeryon_dsp_center_hann_f64(
            input: *const f64,
            input_length: usize,
            output: *mut f64,
            output_length: usize,
        ) -> AeryonDspStatus;
    }

    // Keep c_void referenced so clippy does not flag unused imports through
    // binding churn if signatures evolve.
    #[allow(dead_code)]
    pub type Opaque = *mut c_void;
    #[allow(dead_code)]
    pub type CInt = c_int;
}

/// Reads the compiled native ABI version.
pub fn abi_version() -> i32 {
    // SAFETY: `aeryon_dsp_abi_version` is a pure C function with no pointer args.
    unsafe { bindings::aeryon_dsp_abi_version() }
}

/// Validates that the linked native library ABI matches this crate.
pub fn ensure_abi_compatible() -> Result<(), FfiError> {
    let actual = abi_version();
    if actual != EXPECTED_ABI_VERSION {
        return Err(FfiError::AbiMismatch {
            expected: EXPECTED_ABI_VERSION,
            actual,
        });
    }
    Ok(())
}

/// Reads native library version components.
pub fn library_version() -> Result<NativeLibraryVersion, FfiError> {
    let mut major = 0_i32;
    let mut minor = 0_i32;
    let mut patch = 0_i32;
    // SAFETY: pointers refer to stack locals that remain valid for the call.
    let status = unsafe {
        bindings::aeryon_dsp_library_version(&raw mut major, &raw mut minor, &raw mut patch)
    };
    map_status(
        status,
        "library_version",
        "reading native library version".to_owned(),
    )?;
    Ok(NativeLibraryVersion {
        major,
        minor,
        patch,
    })
}

/// Runs the native per-link motion-energy kernel.
///
/// Input layout is flattened `[frame][subcarrier]` for one antenna link.
/// Output length is `frame_count - 1`.
pub fn motion_energy_f32(
    real_samples: &[f32],
    imag_samples: &[f32],
    frame_count: usize,
    subcarrier_count: usize,
) -> Result<Vec<f64>, FfiError> {
    ensure_abi_compatible()?;

    if frame_count < 2 || subcarrier_count == 0 {
        return Err(FfiError::Native {
            kernel: "motion_energy",
            status: NativeStatus::InvalidLength,
            status_label: NativeStatus::InvalidLength.as_str(),
            context: format!("frame_count={frame_count} subcarrier_count={subcarrier_count}"),
        });
    }

    let expected_len =
        frame_count
            .checked_mul(subcarrier_count)
            .ok_or_else(|| FfiError::Native {
                kernel: "motion_energy",
                status: NativeStatus::InvalidLength,
                status_label: NativeStatus::InvalidLength.as_str(),
                context: format!(
                    "overflow computing sample count frame_count={frame_count} \
                 subcarrier_count={subcarrier_count}"
                ),
            })?;

    if real_samples.len() != expected_len || imag_samples.len() != expected_len {
        return Err(FfiError::Native {
            kernel: "motion_energy",
            status: NativeStatus::DimensionMismatch,
            status_label: NativeStatus::DimensionMismatch.as_str(),
            context: format!(
                "expected {expected_len} samples, real={}, imag={}",
                real_samples.len(),
                imag_samples.len()
            ),
        });
    }

    let out_len = frame_count - 1;
    let mut output = vec![0.0_f64; out_len];

    // SAFETY: lengths and non-null slices were validated above; output buffer is
    // exactly `out_len` and remains alive for the duration of the call. The C ABI
    // does not retain pointers after return.
    let status = unsafe {
        bindings::aeryon_dsp_motion_energy_f32(
            real_samples.as_ptr(),
            imag_samples.as_ptr(),
            frame_count,
            subcarrier_count,
            output.as_mut_ptr(),
            output.len(),
        )
    };
    map_status(
        status,
        "motion_energy",
        format!("frames={frame_count} subcarriers={subcarrier_count}"),
    )?;
    Ok(output)
}

/// Runs the native mean-removal + Hann kernel.
pub fn center_hann_f64(input: &[f64]) -> Result<Vec<f64>, FfiError> {
    ensure_abi_compatible()?;

    if input.is_empty() {
        return Err(FfiError::Native {
            kernel: "center_hann",
            status: NativeStatus::InvalidLength,
            status_label: NativeStatus::InvalidLength.as_str(),
            context: "empty input".to_owned(),
        });
    }

    let mut output = vec![0.0_f64; input.len()];

    // SAFETY: input/output lengths match and buffers stay alive for the call.
    // The C ABI writes only within `output` and retains no pointers.
    let status = unsafe {
        bindings::aeryon_dsp_center_hann_f64(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            output.len(),
        )
    };
    map_status(status, "center_hann", format!("length={}", input.len()))?;
    Ok(output)
}

fn map_status(raw: i32, kernel: &'static str, context: String) -> Result<(), FfiError> {
    let status = NativeStatus::from_raw(raw);
    if status == NativeStatus::Ok {
        return Ok(());
    }
    Err(FfiError::Native {
        kernel,
        status,
        status_label: status.as_str(),
        context,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_is_compatible() {
        ensure_abi_compatible().expect("abi");
        let version = library_version().expect("version");
        assert_eq!(version.major, 0);
        assert_eq!(version.display(), "0.1.0");
    }

    #[test]
    fn motion_energy_known_difference() {
        let real = [1.0_f32, 0.0];
        let imag = [0.0_f32, 1.0];
        let out = motion_energy_f32(&real, &imag, 2, 1).expect("motion");
        assert_eq!(out.len(), 1);
        assert!((out[0] - std::f64::consts::SQRT_2).abs() < 1e-12);
    }

    #[test]
    fn center_hann_constant_is_zero() {
        let out = center_hann_f64(&[3.0; 8]).expect("center");
        assert!(out.iter().all(|v| v.abs() < 1e-15));
    }

    #[test]
    fn rejects_empty_center() {
        let err = center_hann_f64(&[]).expect_err("empty");
        match err {
            FfiError::Native {
                status: NativeStatus::InvalidLength,
                ..
            } => {}
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn rejects_mismatched_motion_length() {
        let err = motion_energy_f32(&[1.0], &[0.0, 1.0], 2, 1).expect_err("mismatch");
        match err {
            FfiError::Native {
                status: NativeStatus::DimensionMismatch,
                ..
            } => {}
            other => panic!("unexpected {other:?}"),
        }
    }
}
