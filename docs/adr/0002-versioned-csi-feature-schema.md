# ADR 0002: Versioned CSI Channel Feature Schema

## Status

Accepted (Milestone M4.1)

## Context

DSP produces motion-energy series and one-sided spectra. Downstream stages need a
stable numerical descriptor layout for inspection, regression testing, and later
optional ML inputs (for example ONNX). Feature layout must not depend on hash-map
iteration order.

## Decision

Define one versioned schema, `csi-channel-features-v1`, with:

- strongly typed feature identifiers
- a fixed ordered layout
- units/semantic descriptions per feature
- aggregate and per-link values derived from existing DSP aggregate series when available
- provenance that records feature, DSP, calibration, and backend identities

Feature extraction is profiled by `baseline-features-v1`. The first observation
consumer (`channel-change-v1`) uses a documented heuristic score over
motion-energy RMS and p95. The score is not a probability and does not claim
human presence or activity.

## Consequences

- Replay comparison and API inspection can rely on stable indices and names.
- Later models may consume the same schema without changing the extractor interface.
- Rust and C++ DSP backends must preserve motion-energy and spectral semantics so
  features remain equivalent within floating-point tolerances.
- Occupancy, activity recognition, and entity tracking remain out of scope until
  richer observations exist.
