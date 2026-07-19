//! Shared domain [`Event`] → [`ApiEventEnvelope`] mapping for WebSocket and REST.

use aeryon_domain::{CsiReplayFailureKind, Event, SensorFailureKind};
use serde_json::json;

use super::dto::{
    ApiEventEnvelope, CalibrationFailedPayload, CalibrationServiceStoppedPayload,
    CalibrationStartedPayload, CsiFrameCalibratedPayload, CsiFramePayload,
    CsiReplayLifecyclePayload, CsiWindowAssembledPayload, DspProcessingFailedPayload,
    DspServiceIdlePayload, DspServiceStartedPayload, DspServiceStoppedPayload,
    DspWindowProcessedPayload, SensorFramePayload, SensorLifecyclePayload,
};
use super::time::{nanos_to_rfc3339, now_rfc3339};

const CSI_DATA_CLASSIFICATION: &str = "deterministic_development_fixture";
const PIPELINE_DATA_CLASSIFICATION: &str = "csi_replay_development_source";

/// Maps a domain event into the versioned API envelope used by WebSocket and REST.
///
/// Returns `None` for event variants that are intentionally not surfaced.
pub fn domain_event_to_envelope(
    event: Event,
    samples_per_frame: usize,
) -> Option<ApiEventEnvelope> {
    match event {
        Event::FrameReceived(frame) => {
            let payload = SensorFramePayload {
                sensor_id: frame.sensor_id.value(),
                sequence: frame.sequence,
                frame_id: frame.frame_id.value(),
                capture_timestamp: nanos_to_rfc3339(frame.timestamp.as_nanos()),
                samples_per_frame,
                source_type: "synthetic",
            };
            Some(ApiEventEnvelope {
                version: 1,
                event_type: "sensor_frame".to_owned(),
                timestamp: now_rfc3339(),
                payload: serde_json::to_value(payload).unwrap_or_else(|_| json!({})),
            })
        }
        Event::CsiFrameReceived(frame) => {
            let payload = CsiFramePayload {
                sensor_id: frame.sensor_id.value(),
                sequence: frame.sequence,
                frame_id: frame.frame_id.value(),
                capture_timestamp: nanos_to_rfc3339(frame.capture_timestamp.as_nanos()),
                receive_timestamp: nanos_to_rfc3339(frame.receive_timestamp.as_nanos()),
                receive_antennas: frame.receive_antennas,
                transmit_antennas: frame.transmit_antennas,
                subcarrier_count: frame.subcarrier_count,
                center_frequency_hz: frame.center_frequency_hz,
                bandwidth_hz: frame.bandwidth_hz,
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                live_hardware: false,
            };
            Some(ApiEventEnvelope {
                version: 1,
                event_type: "csi_frame".to_owned(),
                timestamp: now_rfc3339(),
                payload: serde_json::to_value(payload).unwrap_or_else(|_| json!({})),
            })
        }
        Event::SensorStarted(event) => Some(lifecycle_envelope(
            "sensor_started",
            SensorLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                kind: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::SensorStopped(event) => Some(lifecycle_envelope(
            "sensor_stopped",
            SensorLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                kind: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::SensorFailed(event) => Some(lifecycle_envelope(
            "sensor_failed",
            SensorLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                kind: Some(failure_kind_label(event.kind)),
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CsiReplayStarted(event) => Some(csi_lifecycle_envelope(
            "csi_replay_started",
            CsiReplayLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                kind: None,
                frames_accepted: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CsiReplayCompleted(event) => Some(csi_lifecycle_envelope(
            "csi_replay_completed",
            CsiReplayLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                kind: None,
                frames_accepted: Some(event.frames_accepted),
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CsiReplayStopped(event) => Some(csi_lifecycle_envelope(
            "csi_replay_stopped",
            CsiReplayLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                kind: None,
                frames_accepted: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CsiReplayFailed(event) => Some(csi_lifecycle_envelope(
            "csi_replay_failed",
            CsiReplayLifecyclePayload {
                sensor_id: event.sensor_id.value(),
                source_type: "csi_replay",
                data_classification: CSI_DATA_CLASSIFICATION,
                kind: Some(csi_failure_kind_label(event.kind)),
                frames_accepted: None,
            },
            nanos_to_rfc3339(event.timestamp.as_nanos()),
        )),
        Event::CalibrationStarted(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "calibration_started".to_owned(),
            timestamp: nanos_to_rfc3339(event.timestamp.as_nanos()),
            payload: serde_json::to_value(CalibrationStartedPayload {
                profile_id: event.profile_id,
                profile_version: event.profile_version,
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        Event::CsiFrameCalibrated(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "csi_frame_calibrated".to_owned(),
            timestamp: nanos_to_rfc3339(event.calibrated_at.as_nanos()),
            payload: serde_json::to_value(CsiFrameCalibratedPayload {
                raw_frame_id: event.raw_frame_id.value(),
                sensor_id: event.sensor_id.value(),
                sequence: event.sequence,
                profile_id: event.profile_id,
                profile_version: event.profile_version,
                stage_count: event.stage_count,
                calibration_duration_ns: event.calibration_duration_ns,
                receive_antennas: event.receive_antennas,
                transmit_antennas: event.transmit_antennas,
                subcarrier_count: event.subcarrier_count,
                source_type: event.source.as_str(),
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        Event::CalibrationFailed(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "calibration_failed".to_owned(),
            timestamp: nanos_to_rfc3339(event.timestamp.as_nanos()),
            payload: serde_json::to_value(CalibrationFailedPayload {
                code: event.code.as_str().to_owned(),
                message: event.message,
                raw_frame_id: event.raw_frame_id.map(|id| id.value()),
                sequence: event.sequence,
                failed_stage: event.failed_stage,
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        Event::CalibrationServiceStopped(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "calibration_service_stopped".to_owned(),
            timestamp: nanos_to_rfc3339(event.timestamp.as_nanos()),
            payload: serde_json::to_value(CalibrationServiceStoppedPayload {
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        Event::DspServiceStarted(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "dsp_service_started".to_owned(),
            timestamp: nanos_to_rfc3339(event.timestamp.as_nanos()),
            payload: serde_json::to_value(DspServiceStartedPayload {
                profile_id: event.profile_id,
                profile_version: event.profile_version,
                window_size_frames: event.window_size_frames,
                hop_size_frames: event.hop_size_frames,
                backend_id: event.backend_id,
                backend_version: event.backend_version,
                backend_abi_version: event.backend_abi_version,
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        Event::CsiWindowAssembled(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "csi_window_assembled".to_owned(),
            timestamp: nanos_to_rfc3339(event.timestamp.as_nanos()),
            payload: serde_json::to_value(CsiWindowAssembledPayload {
                window_id: event.window_id,
                sensor_id: event.sensor_id.value(),
                first_sequence: event.first_sequence,
                last_sequence: event.last_sequence,
                frame_count: event.frame_count,
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        Event::DspWindowProcessed(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "dsp_window_processed".to_owned(),
            timestamp: nanos_to_rfc3339(event.processed_at.as_nanos()),
            payload: serde_json::to_value(DspWindowProcessedPayload {
                window_id: event.window_id,
                sensor_id: event.sensor_id.value(),
                first_sequence: event.first_sequence,
                last_sequence: event.last_sequence,
                frame_count: event.frame_count,
                profile_id: event.profile_id,
                profile_version: event.profile_version,
                processing_duration_ns: event.processing_duration_ns,
                effective_sample_rate_hz: event.effective_sample_rate_hz,
                timestamp_jitter: event.timestamp_jitter,
                dominant_non_dc_hz: event.dominant_non_dc_hz,
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        Event::DspProcessingFailed(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "dsp_processing_failed".to_owned(),
            timestamp: nanos_to_rfc3339(event.timestamp.as_nanos()),
            payload: serde_json::to_value(DspProcessingFailedPayload {
                code: event.code.as_str().to_owned(),
                message: event.message,
                window_id: event.window_id,
                sensor_id: event.sensor_id.map(|id| id.value()),
                first_sequence: event.first_sequence,
                last_sequence: event.last_sequence,
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        Event::DspServiceIdle(event) => {
            let event_type = if event.completed {
                "dsp_service_completed"
            } else {
                "dsp_service_idle"
            };
            Some(ApiEventEnvelope {
                version: 1,
                event_type: event_type.to_owned(),
                timestamp: nanos_to_rfc3339(event.timestamp.as_nanos()),
                payload: serde_json::to_value(DspServiceIdlePayload {
                    completed: event.completed,
                    data_classification: PIPELINE_DATA_CLASSIFICATION,
                })
                .unwrap_or_else(|_| json!({})),
            })
        }
        Event::DspServiceStopped(event) => Some(ApiEventEnvelope {
            version: 1,
            event_type: "dsp_service_stopped".to_owned(),
            timestamp: nanos_to_rfc3339(event.timestamp.as_nanos()),
            payload: serde_json::to_value(DspServiceStoppedPayload {
                data_classification: PIPELINE_DATA_CLASSIFICATION,
            })
            .unwrap_or_else(|_| json!({})),
        }),
        _ => None,
    }
}

fn lifecycle_envelope(
    event_type: &str,
    payload: SensorLifecyclePayload,
    timestamp: String,
) -> ApiEventEnvelope {
    ApiEventEnvelope {
        version: 1,
        event_type: event_type.to_owned(),
        timestamp,
        payload: serde_json::to_value(payload).unwrap_or_else(|_| json!({})),
    }
}

fn csi_lifecycle_envelope(
    event_type: &str,
    payload: CsiReplayLifecyclePayload,
    timestamp: String,
) -> ApiEventEnvelope {
    ApiEventEnvelope {
        version: 1,
        event_type: event_type.to_owned(),
        timestamp,
        payload: serde_json::to_value(payload).unwrap_or_else(|_| json!({})),
    }
}

fn failure_kind_label(kind: SensorFailureKind) -> &'static str {
    match kind {
        SensorFailureKind::ProducerExited => "producer_exited",
        SensorFailureKind::PublishFailed => "publish_failed",
    }
}

fn csi_failure_kind_label(kind: CsiReplayFailureKind) -> &'static str {
    match kind {
        CsiReplayFailureKind::FixtureError => "fixture_error",
        CsiReplayFailureKind::MalformedFrame => "malformed_frame",
        CsiReplayFailureKind::PublishFailed => "publish_failed",
        CsiReplayFailureKind::ProducerExited => "producer_exited",
    }
}
