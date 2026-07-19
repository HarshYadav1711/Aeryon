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
    csi/             Canonical WiFi CSI frames and fixture format
    dsp/             DSP orchestration (calls native kernels)
    features/        Feature extraction
    inference/       Model execution
    perception/      Multi-sensor fusion
    world/           World model state and history
    events/          Event bus and subscriptions
    plugins/         Sensor backend plugin interface
    storage/         Persistence layer
    config/          Configuration management
    domain/          Shared domain contracts
    plugin-runtime/  Plugin lifecycle and registry
    runtime/         Application runtime

plugins/
    synthetic-sensor/  Deterministic dual-sine test sensor (not a real sensor)
    csi-replay/        Deterministic CSI fixture replay (not live RF)

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

Milestone M3.1 is implemented: a canonical WiFi CSI frame model, versioned development fixture format, CSI replay sensor plugin, REST/WebSocket metadata, and dashboard source visibility. Replay data is deterministic fixture content — not live RF sensing. Perception algorithms, DSP, calibration, and hardware capture are not implemented. See [ROADMAP.md](ROADMAP.md).

## Development

### Prerequisites

- Rust stable (edition 2024) with `rustfmt` and `clippy` components
- CMake 3.16+
- Python 3.11+
- Node.js 22+

### Local dashboard (two processes)

Terminal 1 — start the Aeryon server (default: synthetic sensor + API):

```bash
cargo run --bin server
```

To use the CSI fixture replay source instead, edit `config/aeryon.toml`:

```toml
[synthetic_sensor]
enabled = false

[sensors.csi_replay]
enabled = true
path = "datasets/fixtures/csi/synthetic_dev_v1.ndjson"
loop_playback = false
frame_interval_ms = 100
maximum_frames = 0
```

Only one of `synthetic_sensor` or `sensors.csi_replay` may be enabled.

Terminal 2 — start the Vite frontend:

```bash
cd frontend
cp .env.example .env   # once
npm install
npm run dev
```

Open `http://127.0.0.1:5173`. Values come from the running Rust server. CSI replay is labeled as development fixture data, not live WiFi CSI.

### API summary (local development)

Configured in `[api]` of `config/aeryon.toml` (default `127.0.0.1:8080`). The API is local-development infrastructure at this stage.

| Endpoint | Purpose |
|----------|---------|
| `GET /health` | Runtime health (200 healthy / 503 failed) |
| `GET /api/v1/runtime` | Runtime statistics snapshot |
| `GET /api/v1/plugins` | Registered plugin summaries |
| `GET /api/v1/sensors/synthetic` | Synthetic sensor snapshot |
| `GET /api/v1/sensors/csi-replay` | CSI fixture replay snapshot |
| `GET /api/v1/events/ws` | WebSocket stream of typed event metadata |

Development CSI fixtures live under `datasets/fixtures/csi/` (see that directory’s README). The format is **Aeryon CSI Fixture Format v1** and is not the production recording format.

Frontend env (`frontend/.env.example`):

- `VITE_AERYON_API_URL=http://127.0.0.1:8080`
- `VITE_AERYON_WS_URL=ws://127.0.0.1:8080`


### Commands

Run from the repository root unless noted.

| Task | Command |
|------|---------|
| Rust tests | `cargo test` or `scripts/cargo-test.ps1` |
| Rust format | `cargo fmt --all` or `scripts/cargo-fmt.ps1` |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Run server | `cargo run --bin server` |
| CLI info | `cargo run -p aeryon-cli -- info` |
| C++ build and test | `scripts/cmake-build.ps1` |
| Python install | `python -m pip install ./ml` or `scripts/python-install.ps1` |
| ML CLI | `aeryon-ml` |
| Frontend install | `cd frontend && npm install` |
| Frontend dev server | `cd frontend && npm run dev` |
| Frontend tests | `cd frontend && npm test` |
| Verify all components | `scripts/verify-all.ps1` |

Unix equivalents are available under `scripts/*.sh`.

With the default `config/aeryon.toml`, the server starts the synthetic sensor, serves the local API, logs frame progress periodically, and shuts down cleanly on Ctrl+C. Set `[synthetic_sensor] enabled = false` or `[api] enabled = false` to disable those pieces.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the full text.
