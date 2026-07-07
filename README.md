# Aeryon

Aeryon is an open-source environmental perception engine. It transforms raw sensor signals into structured, explainable, and reproducible world models.

## Vision

Environmental perception requires more than signal processing in isolation. Aeryon provides a modular platform where acquisition, calibration, feature extraction, inference, and world-model construction are explicit, composable stages. Each stage produces inspectable artifacts with defined interfaces, enabling reproducible research and production deployment from the same codebase.

The first supported sensing backend is WiFi CSI. Future sensor plugins may include radar, UWB, LiDAR, thermal sensors, and scientific instruments. Aeryon is a perception platform—not a WiFi sensing application and not an AI application.

## Architecture Overview

Aeryon is organized as a pipeline of loosely coupled subsystems connected through stable interfaces:

```
Sensor Plugins → Acquisition → Calibration → DSP → Features → Inference → Perception → World Model
                                                                                    ↓
                                                                              Events / Storage
```

- **Acquisition** ingests raw sensor frames from hardware or recorded datasets.
- **Calibration** applies sensor-specific corrections and normalization.
- **DSP** performs high-performance signal processing (C++ with Rust FFI).
- **Features** extracts structured representations from processed signals.
- **Inference** runs deterministic or learned models over feature streams.
- **Perception** fuses multi-sensor outputs into scene-level interpretations.
- **World** maintains the structured world model and its revision history.
- **Events** publishes state changes to subscribers.
- **Plugins** provides the extension interface for new sensor backends.

Applications (`server`, `cli`) and the `frontend` consume the world model and event stream. The `ml/` tree supports offline dataset preparation, training, evaluation, and model export.

## Technology Stack

| Layer | Language | Role |
|-------|----------|------|
| Platform core | Rust | Orchestration, interfaces, storage, plugins |
| Signal processing | C++ | High-performance DSP kernels |
| Research / training | Python | Datasets, training, evaluation, notebooks |
| User interface | React + TypeScript | Visualization and operator tooling |

## Repository Layout

```
apps/
    server/          HTTP/gRPC service entry point
    cli/             Command-line interface

core/
    acquisition/     Sensor frame ingestion
    calibration/     Sensor correction and normalization
    dsp/             DSP orchestration (calls native kernels)
    features/        Feature extraction
    inference/       Model execution
    perception/      Multi-sensor fusion
    world/           World model state and history
    events/          Event bus and subscriptions
    plugins/         Sensor backend plugin interface
    storage/         Persistence layer
    config/          Configuration management

native/
    cpp-dsp/         C++ DSP implementations
    ffi/             Foreign-function interface bindings

ml/
    src/               Python packages (datasets, training, evaluation, export)
    notebooks/         Exploratory analysis

frontend/            Web UI

docs/
    adr/             Architecture Decision Records

benchmarks/          Performance benchmarks
examples/            Usage examples
datasets/            Dataset directory (contents not tracked)
tests/               Integration and system tests
scripts/             Build, CI, and maintenance scripts
```

## Development Philosophy

1. **Interfaces before implementations.** Subsystem boundaries are defined early and changed deliberately via ADRs.
2. **Reproducibility by default.** Pipelines produce versioned artifacts with provenance metadata.
3. **Explainability over opacity.** World-model updates are traceable to source signals and processing steps.
4. **Plugin extensibility.** New sensor backends integrate through the plugin interface without modifying core logic.
5. **Separation of concerns.** DSP, ML training, and platform orchestration live in distinct trees with explicit integration points.

## Current Status

The repository is a buildable multi-language workspace. Subsystems expose minimal public APIs; perception algorithms, DSP kernels, and ML pipelines are not implemented yet. See [ROADMAP.md](ROADMAP.md) for planned milestones.

## Development

### Prerequisites

- Rust stable (edition 2024) with `rustfmt` and `clippy` components
- CMake 3.16+
- Python 3.11+
- Node.js 22+

### Commands

Run from the repository root unless noted.

| Task | Command |
|------|---------|
| Rust tests | `cargo test` or `scripts/cargo-test.ps1` |
| Rust format | `cargo fmt --all` or `scripts/cargo-fmt.ps1` |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| C++ build and test | `scripts/cmake-build.ps1` |
| Python install | `python -m pip install ./ml` or `scripts/python-install.ps1` |
| ML CLI | `aeryon-ml` |
| Frontend install | `cd frontend && npm install` |
| Frontend dev server | `cd frontend && npm run dev` |
| Verify all components | `scripts/verify-all.ps1` |

Unix equivalents are available under `scripts/*.sh`.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the full text.
