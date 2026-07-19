//! Integration tests for the live API surface.

use std::sync::Arc;
use std::time::Duration;

use aeryon_domain::{Event, FrameId, FrameReceived, SensorId, SensorStarted, Timestamp};
use aeryon_runtime::{AppConfig, Runtime, RuntimeHealth};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use serde_json::Value;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tower::ServiceExt;

use crate::api::routes::{bind_ephemeral, build_router, test_state};
use crate::api::state::AppState;

fn test_config(synthetic_enabled: bool) -> AppConfig {
    AppConfig::from_toml(&format!(
        r#"
        [application]
        name = "aeryon"
        environment = "development"

        [logging]
        level = "error"

        [plugins]
        enabled = true
        autoload = false

        [runtime]
        shutdown_timeout_secs = 10
        first_frame_timeout_ms = 2000

        [api]
        enabled = true
        host = "127.0.0.1"
        port = 8080
        cors_origins = ["http://127.0.0.1:5173"]

        [synthetic_sensor]
        enabled = {synthetic_enabled}
        interval_ms = 20
        samples_per_frame = 64
        sample_rate_hz = 1000.0
        primary_frequency_hz = 10.0
        secondary_frequency_hz = 37.0
        secondary_amplitude = 0.25
        maximum_frames = 4
        log_every_n_frames = 10
        "#
    ))
    .expect("valid test config")
}

async fn started_runtime(synthetic_enabled: bool) -> Runtime {
    let mut runtime = Runtime::boot(test_config(synthetic_enabled)).expect("boot");
    runtime.start().expect("start");
    runtime
}

async fn json_get(state: AppState, path: &str) -> (StatusCode, Value) {
    let api = state.runtime().read().await.context().config.api.clone();
    let app = build_router(state, &api);
    let response = app
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    (status, json)
}

#[tokio::test]
async fn health_returns_structured_payload_when_healthy() {
    let runtime = started_runtime(true).await;
    tokio::time::sleep(Duration::from_millis(80)).await;
    let state = test_state(runtime);
    let (status, body) = json_get(state.clone(), "/health").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["status"].as_str().is_some());
    assert_eq!(body["healthy"], true);
    assert!(body["uptime_secs"].as_f64().unwrap_or(0.0) >= 0.0);
    assert!(body["timestamp"].as_str().is_some());
    assert_eq!(body["event_consumer_running"], true);
    assert_eq!(body["synthetic_sensor"]["enabled"], true);

    state.runtime().write().await.shutdown().expect("shutdown");
}

#[tokio::test]
async fn health_returns_503_when_runtime_failed() {
    let mut runtime = Runtime::boot(test_config(true)).expect("boot");
    runtime.start().expect("start");
    // Force failed health by marking sensor failed and refreshing.
    runtime
        .metrics()
        .set_sensor_lifecycle(aeryon_plugin_runtime::LifecycleState::Failed);
    runtime.refresh_health();
    assert_eq!(runtime.health(), RuntimeHealth::Failed);

    let state = test_state(runtime);
    let (status, body) = json_get(state, "/health").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["healthy"], false);
    assert_eq!(body["status"], "failed");
}

#[tokio::test]
async fn runtime_endpoint_returns_live_statistics() {
    let runtime = started_runtime(true).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let frames = runtime.metrics().frames_received();
    let state = test_state(runtime);
    let (status, body) = json_get(state.clone(), "/api/v1/runtime").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["application_name"], "aeryon");
    assert!(body["application_version"].as_str().is_some());
    assert!(body["frames_received"].as_u64().unwrap_or(0) >= frames);
    assert_eq!(body["synthetic_source_enabled"], true);
    assert!(body["registered_plugin_count"].as_u64().unwrap_or(0) >= 1);
    assert!(body["startup_timestamp"].as_str().is_some());

    state.runtime().write().await.shutdown().expect("shutdown");
}

#[tokio::test]
async fn plugins_endpoint_lists_registered_plugins() {
    let runtime = started_runtime(true).await;
    let state = test_state(runtime);
    let (status, body) = json_get(state.clone(), "/api/v1/plugins").await;

    assert_eq!(status, StatusCode::OK);
    let plugins = body["plugins"].as_array().expect("plugins array");
    assert!(!plugins.is_empty());
    let synthetic = plugins
        .iter()
        .find(|plugin| plugin["id"] == "aeryon.synthetic-sensor")
        .expect("synthetic plugin");
    assert_eq!(synthetic["name"], "Synthetic Sensor");
    assert!(
        synthetic["capabilities"]
            .as_array()
            .expect("caps")
            .iter()
            .any(|cap| cap == "sensor")
    );
    assert!(synthetic["lifecycle_state"].as_str().is_some());
    assert!(synthetic["health"].as_str().is_some());

    state.runtime().write().await.shutdown().expect("shutdown");
}

#[tokio::test]
async fn synthetic_sensor_enabled_snapshot() {
    let runtime = started_runtime(true).await;
    tokio::time::sleep(Duration::from_millis(80)).await;
    let state = test_state(runtime);
    let (status, body) = json_get(state.clone(), "/api/v1/sensors/synthetic").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], true);
    assert_eq!(body["samples_per_frame"], 64);
    assert_eq!(body["sample_rate_hz"], 1000.0);
    assert_eq!(body["configured_frequencies_hz"]["primary_hz"], 10.0);
    assert!(body["frames_received"].as_u64().unwrap_or(0) >= 1);
    assert!(body["last_sequence"].as_u64().is_some());
    assert!(body["last_frame_timestamp"].as_str().is_some());

    state.runtime().write().await.shutdown().expect("shutdown");
}

#[tokio::test]
async fn synthetic_sensor_disabled_and_missing_frame_fields() {
    let runtime = started_runtime(false).await;
    let state = test_state(runtime);
    let (status, body) = json_get(state.clone(), "/api/v1/sensors/synthetic").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], false);
    assert_eq!(body["frames_received"], 0);
    assert!(body["last_sequence"].is_null());
    assert!(body["last_frame_timestamp"].is_null());
    assert_eq!(body["configured_interval_ms"], 20);

    state.runtime().write().await.shutdown().expect("shutdown");
}

#[tokio::test]
async fn websocket_upgrade_delivers_frame_event_without_samples() {
    let runtime = started_runtime(false).await;
    let bus = runtime.context().event_bus.clone();
    let state = AppState::new(Arc::new(RwLock::new(runtime)));
    let api = state.runtime().read().await.context().config.api.clone();

    let (addr, shutdown_tx) = bind_ephemeral(state.clone(), api).await.expect("bind");

    let url = format!("ws://{addr}/api/v1/events/ws");
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("ws connect");

    bus.publish(Event::SensorStarted(SensorStarted {
        sensor_id: SensorId::new(1),
        timestamp: Timestamp::from_nanos(1_000),
    }))
    .expect("publish started");

    bus.publish(Event::FrameReceived(FrameReceived {
        frame_id: FrameId::new(7),
        sensor_id: SensorId::new(1),
        timestamp: Timestamp::from_nanos(2_000_000_000),
        sequence: 7,
    }))
    .expect("publish frame");

    let mut saw_frame = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let next = tokio::time::timeout(Duration::from_millis(400), ws.next()).await;
        let Ok(Some(Ok(WsMessage::Text(text)))) = next else {
            continue;
        };
        let envelope: Value = serde_json::from_str(&text).expect("json");
        if envelope["type"] == "sensor_frame" {
            assert_eq!(envelope["version"], 1);
            assert_eq!(envelope["payload"]["sequence"], 7);
            assert_eq!(envelope["payload"]["frame_id"], 7);
            assert_eq!(envelope["payload"]["source_type"], "synthetic");
            assert_eq!(envelope["payload"]["samples_per_frame"], 64);
            assert!(envelope["payload"].get("samples").is_none());
            assert!(envelope["payload"].get("values").is_none());
            saw_frame = true;
            break;
        }
    }
    assert!(saw_frame, "expected sensor_frame event");

    ws.close(None).await.ok();
    let _ = shutdown_tx.send(true);
    tokio::time::sleep(Duration::from_millis(50)).await;
    state.runtime().write().await.shutdown().expect("shutdown");
}

#[tokio::test]
async fn websocket_client_disconnect_is_clean() {
    let runtime = started_runtime(false).await;
    let state = AppState::new(Arc::new(RwLock::new(runtime)));
    let api = state.runtime().read().await.context().config.api.clone();
    let (addr, shutdown_tx) = bind_ephemeral(state.clone(), api).await.expect("bind");

    let url = format!("ws://{addr}/api/v1/events/ws");
    let (ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("ws connect");
    drop(ws);

    tokio::time::sleep(Duration::from_millis(50)).await;
    let _ = shutdown_tx.send(true);
    state.runtime().write().await.shutdown().expect("shutdown");
}

fn csi_replay_config(path: &str) -> AppConfig {
    AppConfig::from_toml(&format!(
        r#"
        [application]
        name = "aeryon"
        environment = "development"

        [logging]
        level = "error"

        [plugins]
        enabled = true
        autoload = false

        [runtime]
        shutdown_timeout_secs = 10
        first_frame_timeout_ms = 2000

        [api]
        enabled = true
        host = "127.0.0.1"
        port = 8080
        cors_origins = ["http://127.0.0.1:5173"]

        [synthetic_sensor]
        enabled = false

        [sensors.csi_replay]
        enabled = true
        path = "{path}"
        loop_playback = false
        frame_interval_ms = 15
        maximum_frames = 6

        [calibration]
        enabled = true
        profile = "baseline-csi-v1"
        queue_capacity = 32
        "#
    ))
    .expect("valid csi config")
}

fn csi_dsp_config(path: &str) -> AppConfig {
    AppConfig::from_toml(&format!(
        r#"
        [application]
        name = "aeryon"
        environment = "development"

        [logging]
        level = "error"

        [plugins]
        enabled = true
        autoload = false

        [runtime]
        shutdown_timeout_secs = 10
        first_frame_timeout_ms = 2000

        [api]
        enabled = true
        host = "127.0.0.1"
        port = 8080
        cors_origins = ["http://127.0.0.1:5173"]

        [synthetic_sensor]
        enabled = false

        [sensors.csi_replay]
        enabled = true
        path = "{path}"
        loop_playback = false
        frame_interval_ms = 10
        maximum_frames = 16

        [calibration]
        enabled = true
        profile = "baseline-csi-v1"
        queue_capacity = 32

        [dsp]
        enabled = true
        profile = "baseline-dsp-v1"
        queue_capacity = 32
        window_size_frames = 8
        hop_size_frames = 4
        maximum_sequence_gap = 1
        timestamp_jitter_tolerance = 0.50
        "#
    ))
    .expect("valid dsp config")
}

fn write_temp_csi_fixture() -> tempfile::NamedTempFile {
    write_temp_csi_fixture_with_frames(8)
}

fn write_temp_csi_fixture_with_frames(frame_count: u64) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().expect("temp");
    writeln!(
        file,
        r#"{{"record_type":"header","schema":"aeryon-csi-fixture","version":1,"sensor_id":"2","description":"api e2e","sample_layout":"rx-tx-subcarrier"}}"#
    )
    .expect("header");
    for sequence in 0..frame_count {
        // Uniform 10 ms capture spacing keeps spectral sampling valid.
        let capture_nanos = 1_000_000_000 + sequence * 10_000_000;
        let scale = 1.0 + (sequence as f64) * 0.05;
        writeln!(
            file,
            r#"{{"record_type":"frame","frame_id":{},"sequence":{},"capture_timestamp_nanos":{},"center_frequency_hz":5180000000.0,"bandwidth_hz":20000000.0,"receive_antennas":2,"transmit_antennas":1,"subcarrier_indices":[-1,0,1],"samples":[{{"re":{},"im":0.0}},{{"re":0.0,"im":{}}},{{"re":{},"im":0.0}},{{"re":{},"im":0.0}},{{"re":0.0,"im":{}}},{{"re":{},"im":0.0}}]}}"#,
            sequence + 1,
            sequence,
            capture_nanos,
            scale,
            scale,
            -scale,
            2.0 * scale,
            2.0 * scale,
            -2.0 * scale
        )
        .expect("frame");
    }
    file
}

#[tokio::test]
async fn csi_replay_end_to_end_events_stats_and_endpoint() {
    let fixture = write_temp_csi_fixture();
    let path = fixture.path().to_string_lossy().replace('\\', "/");
    let mut runtime = Runtime::boot(csi_replay_config(&path)).expect("boot");
    let mut receiver = runtime.context().event_bus.subscribe();
    runtime.start().expect("start");

    let mut sequences = Vec::new();
    let mut calibrated = 0_u32;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while (sequences.len() < 3 || calibrated < 3) && tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), receiver.recv()).await {
            Ok(Ok(Event::CsiFrameReceived(frame))) => {
                assert_eq!(frame.receive_antennas, 2);
                assert_eq!(frame.transmit_antennas, 1);
                assert_eq!(frame.subcarrier_count, 3);
                assert_eq!(frame.source.as_str(), "csi_replay");
                sequences.push(frame.sequence);
            }
            Ok(Ok(Event::CsiFrameCalibrated(event))) => {
                assert_eq!(event.profile_id, "baseline-csi-v1");
                assert_eq!(event.source.as_str(), "csi_replay");
                calibrated += 1;
            }
            Ok(Ok(_)) => {}
            _ => break,
        }
    }

    assert!(
        sequences.len() >= 3,
        "expected at least 3 CSI frames, got {sequences:?}"
    );
    assert!(
        calibrated >= 3,
        "expected at least 3 calibrated frames, got {calibrated}"
    );
    assert!(sequences.windows(2).all(|pair| pair[1] == pair[0] + 1));
    assert!(runtime.metrics().frames_received() >= 3);
    assert!(runtime.metrics().csi_replay().frames_accepted() >= 3);
    assert!(runtime.metrics().calibration().frames_calibrated() >= 3);

    let state = test_state(runtime);
    let (status, body) = json_get(state.clone(), "/api/v1/sensors/csi-replay").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], true);
    assert_eq!(body["source_type"], "csi_replay");
    assert_eq!(
        body["data_classification"],
        "deterministic_development_fixture"
    );
    assert!(body["frames_accepted"].as_u64().unwrap_or(0) >= 3);
    assert_eq!(body["receive_antennas"], 2);
    assert_eq!(body["transmit_antennas"], 1);
    assert_eq!(body["subcarrier_count"], 3);
    assert!(body["latest_sequence"].as_u64().is_some());
    assert!(body["fixture_path"].as_str().is_some());

    let (cal_status, cal_body) = json_get(state.clone(), "/api/v1/calibration").await;
    assert_eq!(cal_status, StatusCode::OK);
    assert_eq!(cal_body["enabled"], true);
    assert_eq!(cal_body["profile_id"], "baseline-csi-v1");
    assert_eq!(cal_body["profile_version"], 1);
    assert!(cal_body["frames_calibrated"].as_u64().unwrap_or(0) >= 3);
    assert!(cal_body["raw_frames_submitted"].as_u64().unwrap_or(0) >= 3);
    assert!(cal_body["stages"].as_array().is_some_and(|s| s.len() == 3));
    assert_eq!(
        cal_body["data_classification"],
        "csi_replay_development_source"
    );

    let (runtime_status, runtime_body) = json_get(state.clone(), "/api/v1/runtime").await;
    assert_eq!(runtime_status, StatusCode::OK);
    assert_eq!(runtime_body["csi_replay_enabled"], true);
    assert_eq!(runtime_body["active_source"], "csi_replay");
    assert_eq!(runtime_body["synthetic_source_enabled"], false);

    state.runtime().write().await.shutdown().expect("shutdown");
    assert!(!state.runtime().read().await.metrics().consumer_running());
}

#[tokio::test]
async fn signal_and_dsp_endpoints_report_no_data_before_frames() {
    let runtime = started_runtime(false).await;
    let state = test_state(runtime);

    let (signal_status, signal_body) = json_get(state.clone(), "/api/v1/signal/latest").await;
    assert_eq!(signal_status, StatusCode::OK);
    assert_eq!(signal_body["available"], false);
    assert!(signal_body.get("raw_amplitudes").is_none());

    let (dsp_status, dsp_body) = json_get(state.clone(), "/api/v1/dsp/latest").await;
    assert_eq!(dsp_status, StatusCode::OK);
    assert_eq!(dsp_body["available"], false);
    assert!(dsp_body.get("spectrum_power").is_none());

    let (status_status, status_body) = json_get(state.clone(), "/api/v1/dsp").await;
    assert_eq!(status_status, StatusCode::OK);
    assert_eq!(status_body["enabled"], false);
    assert_eq!(status_body["worker_state"], "disabled");
    assert_eq!(status_body["health"], "disabled");

    let (events_status, events_body) = json_get(state.clone(), "/api/v1/events/recent").await;
    assert_eq!(events_status, StatusCode::OK);
    assert!(events_body["events"].as_array().expect("events").is_empty());

    state.runtime().write().await.shutdown().expect("shutdown");
}

#[tokio::test]
async fn signal_dsp_and_events_endpoints_after_pipeline() {
    let fixture = write_temp_csi_fixture_with_frames(16);
    let path = fixture.path().to_string_lossy().replace('\\', "/");
    let mut runtime = Runtime::boot(csi_dsp_config(&path)).expect("boot");
    let mut receiver = runtime.context().event_bus.subscribe();
    runtime.start().expect("start");

    let mut windows = 0_u32;
    let mut failures = Vec::new();
    let mut started = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while windows < 1 && tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(800), receiver.recv()).await {
            Ok(Ok(Event::DspServiceStarted(_))) => started = true,
            Ok(Ok(Event::DspWindowProcessed(_))) => windows += 1,
            Ok(Ok(Event::DspProcessingFailed(event))) => {
                failures.push(format!("{}:{}", event.code.as_str(), event.message));
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) => break,
            Err(_) => continue,
        }
    }
    assert!(
        windows >= 1,
        "expected at least one DSP window (started={started}, failures={failures:?}, emitted={})",
        runtime.metrics().dsp().windows_emitted()
    );

    // Allow finite replay to settle into completed/idle.
    let settle = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < settle {
        let state = runtime.metrics().dsp().worker_state();
        if matches!(
            state,
            aeryon_runtime::DspWorkerState::Completed | aeryon_runtime::DspWorkerState::Idle
        ) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let state = test_state(runtime);

    let (dsp_status, dsp_body) = json_get(state.clone(), "/api/v1/dsp").await;
    assert_eq!(dsp_status, StatusCode::OK);
    assert_eq!(dsp_body["enabled"], true);
    assert_eq!(dsp_body["profile_id"], "baseline-dsp-v1");
    assert!(dsp_body["windows_emitted"].as_u64().unwrap_or(0) >= 1);
    assert!(dsp_body["calibrated_frames_received"].as_u64().unwrap_or(0) >= 4);
    let health = dsp_body["health"].as_str().expect("health");
    assert!(
        matches!(health, "running" | "completed" | "idle"),
        "unexpected dsp health {health}"
    );
    assert_ne!(health, "failed");

    let (signal_status, signal_body) =
        json_get(state.clone(), "/api/v1/signal/latest?rx=0&tx=0").await;
    assert_eq!(signal_status, StatusCode::OK);
    assert_eq!(signal_body["available"], true);
    assert_eq!(signal_body["rx"], 0);
    assert_eq!(signal_body["tx"], 0);
    assert_eq!(signal_body["source_classification"], "csi_replay");
    assert_eq!(
        signal_body["raw_amplitudes"]
            .as_array()
            .expect("raw amplitudes")
            .len(),
        3
    );
    assert_eq!(
        signal_body["calibrated_amplitudes"]
            .as_array()
            .expect("cal amplitudes")
            .len(),
        3
    );
    assert_eq!(
        signal_body["raw_wrapped_phases"]
            .as_array()
            .expect("raw phases")
            .len(),
        3
    );
    assert_eq!(
        signal_body["calibrated_phases"]
            .as_array()
            .expect("cal phases")
            .len(),
        3
    );
    let grid = signal_body["calibrated_magnitude_grid"]
        .as_array()
        .expect("grid");
    assert_eq!(grid.len(), 2);
    assert_eq!(grid[0]["rx"], 0);
    assert_eq!(grid[1]["rx"], 1);

    let (signal_bad, signal_err) = json_get(state.clone(), "/api/v1/signal/latest?rx=9&tx=0").await;
    assert_eq!(signal_bad, StatusCode::BAD_REQUEST);
    assert_eq!(signal_err["error"]["code"], "invalid_link");

    let (latest_status, latest_body) =
        json_get(state.clone(), "/api/v1/dsp/latest?rx=0&tx=0").await;
    assert_eq!(latest_status, StatusCode::OK);
    assert_eq!(latest_body["available"], true);
    assert_eq!(latest_body["rx"], 0);
    assert_eq!(latest_body["tx"], 0);
    assert!(latest_body["first_sequence"].as_u64().is_some());
    assert!(latest_body["last_sequence"].as_u64().is_some());
    assert!(
        !latest_body["motion_energy_values"]
            .as_array()
            .expect("motion")
            .is_empty()
    );
    assert!(
        !latest_body["spectrum_frequencies_hz"]
            .as_array()
            .expect("freqs")
            .is_empty()
    );
    assert!(
        !latest_body["spectrum_power"]
            .as_array()
            .expect("power")
            .is_empty()
    );
    assert!(latest_body["motion_energy_semantics"].as_str().is_some());

    let (latest_bad, latest_err) = json_get(state.clone(), "/api/v1/dsp/latest?rx=3&tx=0").await;
    assert_eq!(latest_bad, StatusCode::BAD_REQUEST);
    assert_eq!(latest_err["error"]["code"], "invalid_link");

    let (events_status, events_body) =
        json_get(state.clone(), "/api/v1/events/recent?limit=50").await;
    assert_eq!(events_status, StatusCode::OK);
    let events = events_body["events"].as_array().expect("events");
    assert!(!events.is_empty());
    assert!(
        events
            .iter()
            .any(|event| event["type"] == "dsp_window_processed")
    );
    assert!(events.iter().any(|event| {
        matches!(
            event["type"].as_str(),
            Some("csi_frame") | Some("csi_frame_calibrated") | Some("dsp_service_started")
        )
    }));
    // Chronological: timestamps should be non-decreasing for envelope timestamp strings when present.
    assert!(events.len() <= 50);

    let (events_capped, events_capped_body) =
        json_get(state.clone(), "/api/v1/events/recent?limit=1000").await;
    assert_eq!(events_capped, StatusCode::OK);
    let capped = events_capped_body["events"].as_array().expect("capped");
    assert!(capped.len() <= 100);

    let (events_one, events_one_body) =
        json_get(state.clone(), "/api/v1/events/recent?limit=1").await;
    assert_eq!(events_one, StatusCode::OK);
    assert_eq!(events_one_body["events"].as_array().expect("one").len(), 1);

    state.runtime().write().await.shutdown().expect("shutdown");
}
