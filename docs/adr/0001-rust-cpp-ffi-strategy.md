# 0001. Rust/C++ FFI strategy for DSP kernels

Date: 2026-07-20
Status: accepted

## Context

Aeryon needs optional high-performance C++ numerical kernels for DSP while Rust
owns the platform, configuration, runtime, API, and scientifically authoritative
reference implementation. Numerical buffers (motion-energy inputs and centered /
Hann-windowed signals) must cross the language boundary safely.

## Decision

- Expose native kernels through a stable C ABI (`native/cpp-dsp`).
- Use caller-owned buffers with explicit lengths.
- Return integer status codes; do not throw C++ exceptions across the ABI.
- Do not pass C++ object ownership across the boundary.
- Keep a minimal, audited `unsafe` Rust wrapper in `native/ffi` (`aeryon-dsp-ffi`).
- Gate linking behind an optional `cpp-dsp` Cargo feature.
- Retain the pure-Rust reference backend as the default and conformance oracle.

## Alternatives considered

- `cxx` or another generated bridge
- Exposing C++ classes directly to Rust
- Python bindings for the hot path
- An external DSP microservice
- Rust-only implementation (no native path)
- CUDA / GPU kernels

## Consequences

- ABI and status codes require explicit versioning and manual maintenance.
- The surface stays small, transparent, and easy to test for numerical parity.
- Dependency complexity stays low compared with generated binding stacks.
- Interface shapes are restricted to flat buffers and status codes.
- Builds without `cpp-dsp` reject a configured `backend = "cpp"` instead of
  silently falling back to Rust.
