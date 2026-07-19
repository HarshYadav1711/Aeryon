//! Typed DSP numerical-kernel backends (Rust reference and optional C++).

use std::fmt;
use std::sync::Arc;

use serde::Deserialize;

use crate::errors::DspError;
use crate::kernels;

/// Implementation version of the Rust reference kernel backend.
pub const RUST_BACKEND_VERSION: &str = "1.0.0";

/// Implementation version of the C++ native kernel backend (logical package version).
pub const CPP_BACKEND_VERSION: &str = "0.1.0";

/// Expected native ABI version when the C++ backend is linked.
pub const CPP_ABI_VERSION: u32 = 1;

/// Typed DSP kernel backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DspBackendKind {
    /// Pure-Rust reference kernels (default; scientifically authoritative).
    #[default]
    Rust,
    /// Optional C++ numerical kernels behind the C ABI / FFI wrapper.
    Cpp,
}

impl DspBackendKind {
    /// Stable configuration / wire identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Cpp => "cpp",
        }
    }

    /// Human-readable display name for operators and the dashboard.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Rust => "Rust reference backend",
            Self::Cpp => "C++ native backend",
        }
    }

    /// Whether this backend was compiled into the current binary.
    pub const fn is_compiled(self) -> bool {
        match self {
            Self::Rust => true,
            Self::Cpp => cfg!(feature = "cpp-dsp"),
        }
    }
}

impl fmt::Display for DspBackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Provenance / health identity for an instantiated backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DspBackendIdentity {
    /// Selected backend kind.
    pub kind: DspBackendKind,
    /// Implementation version string.
    pub implementation_version: String,
    /// Native ABI version when applicable.
    pub abi_version: Option<u32>,
    /// Whether the backend is available in this build.
    pub build_available: bool,
    /// Human-readable display name.
    pub display_name: String,
}

impl DspBackendIdentity {
    /// Rust reference identity.
    pub fn rust() -> Self {
        Self {
            kind: DspBackendKind::Rust,
            implementation_version: RUST_BACKEND_VERSION.to_owned(),
            abi_version: None,
            build_available: true,
            display_name: DspBackendKind::Rust.display_name().to_owned(),
        }
    }

    /// C++ backend identity (compiled and ABI-compatible).
    pub fn cpp(implementation_version: String, abi_version: u32) -> Self {
        Self {
            kind: DspBackendKind::Cpp,
            implementation_version,
            abi_version: Some(abi_version),
            build_available: DspBackendKind::Cpp.is_compiled(),
            display_name: DspBackendKind::Cpp.display_name().to_owned(),
        }
    }
}

/// Flattened per-link motion-energy input (`[frame][subcarrier]`).
#[derive(Debug, Clone, Copy)]
pub struct MotionEnergyInput<'a> {
    /// Contiguous real parts.
    pub real_samples: &'a [f32],
    /// Contiguous imaginary parts.
    pub imag_samples: &'a [f32],
    /// Frame count (>= 2).
    pub frame_count: usize,
    /// Subcarrier count (>= 1).
    pub subcarrier_count: usize,
}

/// Numerical kernel backend used by the DSP service.
pub trait DspKernelBackend: Send + Sync + 'static {
    /// Backend identity and versions.
    fn identity(&self) -> DspBackendIdentity;

    /// Per-link motion-energy kernel.
    fn motion_energy(&self, input: MotionEnergyInput<'_>) -> Result<Vec<f64>, DspError>;

    /// Mean removal + Hann window.
    fn center_and_apply_hann(&self, signal: &[f64]) -> Result<Vec<f64>, DspError>;
}

/// Pure-Rust reference kernel backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct RustKernelBackend;

impl DspKernelBackend for RustKernelBackend {
    fn identity(&self) -> DspBackendIdentity {
        DspBackendIdentity::rust()
    }

    fn motion_energy(&self, input: MotionEnergyInput<'_>) -> Result<Vec<f64>, DspError> {
        kernels::motion_energy_link(
            input.real_samples,
            input.imag_samples,
            input.frame_count,
            input.subcarrier_count,
        )
    }

    fn center_and_apply_hann(&self, signal: &[f64]) -> Result<Vec<f64>, DspError> {
        kernels::center_and_apply_hann(signal)
    }
}

/// Optional C++ kernel backend (requires the `cpp-dsp` feature).
#[cfg(feature = "cpp-dsp")]
#[derive(Debug, Clone)]
pub struct CppKernelBackend {
    identity: DspBackendIdentity,
}

#[cfg(feature = "cpp-dsp")]
impl CppKernelBackend {
    /// Validates ABI compatibility and constructs the backend.
    pub fn try_new() -> Result<Self, DspError> {
        aeryon_dsp_ffi::ensure_abi_compatible().map_err(|error| DspError::BackendUnavailable {
            backend: DspBackendKind::Cpp,
            message: error.to_string(),
        })?;
        let version =
            aeryon_dsp_ffi::library_version().map_err(|error| DspError::BackendUnavailable {
                backend: DspBackendKind::Cpp,
                message: error.to_string(),
            })?;
        let abi = u32::try_from(aeryon_dsp_ffi::abi_version()).unwrap_or(0);
        if abi != CPP_ABI_VERSION {
            return Err(DspError::BackendUnavailable {
                backend: DspBackendKind::Cpp,
                message: format!("native ABI mismatch: expected {CPP_ABI_VERSION}, got {abi}"),
            });
        }
        Ok(Self {
            identity: DspBackendIdentity::cpp(version.display(), abi),
        })
    }
}

#[cfg(feature = "cpp-dsp")]
impl DspKernelBackend for CppKernelBackend {
    fn identity(&self) -> DspBackendIdentity {
        self.identity.clone()
    }

    fn motion_energy(&self, input: MotionEnergyInput<'_>) -> Result<Vec<f64>, DspError> {
        aeryon_dsp_ffi::motion_energy_f32(
            input.real_samples,
            input.imag_samples,
            input.frame_count,
            input.subcarrier_count,
        )
        .map_err(|error| {
            map_ffi_error(
                "motion_energy",
                error,
                input.frame_count,
                input.subcarrier_count,
            )
        })
    }

    fn center_and_apply_hann(&self, signal: &[f64]) -> Result<Vec<f64>, DspError> {
        aeryon_dsp_ffi::center_hann_f64(signal)
            .map_err(|error| map_ffi_error("center_hann", error, signal.len(), 0))
    }
}

#[cfg(feature = "cpp-dsp")]
fn map_ffi_error(
    kernel: &'static str,
    error: aeryon_dsp_ffi::FfiError,
    dim_a: usize,
    dim_b: usize,
) -> DspError {
    match error {
        aeryon_dsp_ffi::FfiError::AbiMismatch { expected, actual } => {
            DspError::BackendUnavailable {
                backend: DspBackendKind::Cpp,
                message: format!("ABI mismatch: expected {expected}, got {actual}"),
            }
        }
        aeryon_dsp_ffi::FfiError::Native {
            status,
            status_label,
            context,
            ..
        } => DspError::NativeKernel {
            backend: DspBackendKind::Cpp,
            kernel,
            status: status_label.to_owned(),
            message: format!(
                "native status {:?} ({status_label}); dims=({dim_a},{dim_b}); {context}",
                status
            ),
        },
    }
}

/// Instantiates the configured backend, failing clearly when unavailable.
pub fn create_backend(kind: DspBackendKind) -> Result<Arc<dyn DspKernelBackend>, DspError> {
    match kind {
        DspBackendKind::Rust => Ok(Arc::new(RustKernelBackend)),
        DspBackendKind::Cpp => {
            if !DspBackendKind::Cpp.is_compiled() {
                return Err(DspError::BackendUnavailable {
                    backend: DspBackendKind::Cpp,
                    message: "C++ DSP backend requested but this build was compiled without \
                              the `cpp-dsp` Cargo feature"
                        .to_owned(),
                });
            }
            #[cfg(feature = "cpp-dsp")]
            {
                Ok(Arc::new(CppKernelBackend::try_new()?))
            }
            #[cfg(not(feature = "cpp-dsp"))]
            {
                unreachable!("is_compiled() is false without cpp-dsp");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_backend_is_default_and_available() {
        assert_eq!(DspBackendKind::default(), DspBackendKind::Rust);
        assert!(DspBackendKind::Rust.is_compiled());
        let backend = create_backend(DspBackendKind::Rust).expect("rust");
        assert_eq!(backend.identity().kind, DspBackendKind::Rust);
    }

    #[test]
    #[cfg(not(feature = "cpp-dsp"))]
    fn cpp_unavailable_without_feature() {
        let err = match create_backend(DspBackendKind::Cpp) {
            Err(error) => error,
            Ok(_) => panic!("cpp backend should be unavailable without feature"),
        };
        match err {
            DspError::BackendUnavailable {
                backend: DspBackendKind::Cpp,
                ..
            } => {}
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "cpp-dsp")]
    fn cpp_backend_initializes() {
        let backend = create_backend(DspBackendKind::Cpp).expect("cpp");
        let identity = backend.identity();
        assert_eq!(identity.kind, DspBackendKind::Cpp);
        assert_eq!(identity.abi_version, Some(CPP_ABI_VERSION));
    }
}
