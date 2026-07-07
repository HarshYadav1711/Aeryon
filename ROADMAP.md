# Roadmap

This document outlines planned milestones for Aeryon. Items are ordered roughly by dependency. Timelines are not committed.

## Phase 0 — Foundation (current)

- [x] Repository structure and documentation
- [x] Build system and CI pipeline
- [x] Core crate/workspace layout (Rust)
- [ ] Configuration schema
- [ ] Logging and tracing infrastructure

## Phase 1 — Acquisition and Storage

- [ ] Plugin interface definition
- [ ] WiFi CSI acquisition plugin (hardware abstraction only)
- [ ] Frame serialization format
- [ ] Storage layer for raw and processed artifacts
- [ ] CLI for dataset ingestion and replay

## Phase 2 — Signal Processing

- [ ] C++ DSP library scaffolding
- [ ] Rust FFI bindings
- [ ] Calibration pipeline interface
- [ ] Basic DSP operations (filtering, windowing)

## Phase 3 — Features and Inference

- [ ] Feature extraction framework
- [ ] Python training pipeline scaffolding
- [ ] Model export format
- [ ] Inference runtime integration

## Phase 4 — Perception and World Model

- [ ] Perception fusion interface
- [ ] World model schema
- [ ] Event bus
- [ ] Revision history and provenance tracking

## Phase 5 — Applications

- [ ] Server application (API)
- [ ] Frontend for world model visualization
- [ ] Example pipelines and tutorials

## Future Sensor Backends

The plugin architecture is designed to support additional backends without core changes:

- Radar
- UWB
- LiDAR
- Thermal sensors
- Scientific instruments

Each backend will be developed as an independent plugin following the interface defined in Phase 1.

## Out of Scope (for now)

- Cloud deployment infrastructure
- Pre-trained model distribution
- Commercial licensing or hosted services
