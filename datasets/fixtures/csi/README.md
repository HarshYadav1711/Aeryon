# CSI Development Fixtures

This directory contains **deterministic CSI-shaped development data** for Aeryon.

## Important

- Values are mathematically generated and checked in for stable tests.
- They were **not** captured from real WiFi hardware.
- Format: **Aeryon CSI Fixture Format v1** (NDJSON).
- This is **not** the final production recording format.

## Files

| File | Description |
|------|-------------|
| `synthetic_dev_v1.ndjson` | 32 frames, 2 RX × 1 TX × 16 subcarriers |

## Run CSI replay

In `config/aeryon.toml`:

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

Then from the repository root:

```bash
cargo run --bin server
```

Inspect replay state at `GET /api/v1/sensors/csi-replay`. Use the dashboard to confirm the source is labeled as CSI replay development data.
