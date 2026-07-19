# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Milestone M4.1: CSI feature extraction and channel-change observation — `csi-channel-features-v1` / `baseline-features-v1`, immutable `FeatureVector`, deterministic `ChannelChangeObservation` (`channel-change-v1`), feature and perception workers with stats/health, snapshot store slots, REST/WebSocket metadata, Signal Observatory feature and observation panels, and ADR 0002
- API endpoints `GET /api/v1/features`, `GET /api/v1/features/latest`, `GET /api/v1/perception`, `GET /api/v1/observations/latest`
- Milestone M3.4: native DSP backend and benchmark parity — optional C++ kernels behind `cpp-dsp`, typed Rust/C++ backend selection (`backend = "rust"|"cpp"`), FFI-safe C ABI wrappers, Rust↔C++ conformance tests, Criterion backend benches, and ADR 0001 (Rust/C++ FFI strategy)
- Milestone M3.3: temporal CSI windowing and Signal Observatory — `baseline-dsp-v1` motion-energy proxy + Hann/FFT spectra, DSP worker and stats, signal snapshot store, recent event history, REST/WebSocket DSP surfaces, and dashboard charts over deterministic fixture data
- API endpoints `GET /api/v1/dsp`, `GET /api/v1/signal/latest`, `GET /api/v1/dsp/latest`, `GET /api/v1/events/recent`
- Milestone M3.2: configurable CSI calibration pipeline (`aeryon-calibration`) with baseline-csi-v1 stages (spatial phase unwrap, linear phase detrend, RMS amplitude normalize), runtime worker, `GET /api/v1/calibration`, WebSocket calibration metadata events, and dashboard calibration panel
- Bounded CSI frame data path from replay → calibration (event bus remains metadata-only)
- Milestone M3.1: canonical CSI frame (`aeryon-csi`), versioned development fixture format, CSI replay plugin, REST/WebSocket metadata, and dashboard source visibility
- Checked-in synthetic CSI fixture under `datasets/fixtures/csi/` (not hardware-captured; not production recording format)
- API endpoint `GET /api/v1/sensors/csi-replay` and CSI metadata WebSocket events
- Milestone M2.2: live Axum REST/WebSocket API and React dashboard over real runtime state
- API configuration (`[api]` host/port/CORS) in the existing TOML config system
- Local-development endpoints: `/health`, `/api/v1/runtime`, `/api/v1/plugins`, `/api/v1/sensors/synthetic`, `/api/v1/events/ws`
- Milestone M1.1–M1.3: domain contracts, plugin runtime, application runtime
- Milestone M2.1: deterministic synthetic sensor plugin, typed event bus, runtime frame consumer
- Cargo workspace with domain, plugin-runtime, runtime, events, and synthetic-sensor crates
- C++ DSP library scaffold (`native/cpp-dsp`) with CMake build
- Python ML package (`ml/`) with `aeryon-ml` CLI entry point
- React + TypeScript + Vite frontend
- Developer scripts under `scripts/`
- GitHub Actions CI workflow
- EditorConfig, rustfmt, and Clippy configuration
- Project documentation and Apache 2.0 license
