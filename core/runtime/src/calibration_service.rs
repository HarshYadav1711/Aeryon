//! Calibration worker integrated into the application runtime.
//!
//! # Data path vs event path
//!
//! - **Data path:** a bounded `mpsc` channel transports `Arc<CsiFrame>` from the
//!   CSI replay plugin to this worker. Frames are never silently dropped;
//!   producers await capacity under backpressure. Successful calibrations may be
//!   forwarded on a second bounded `mpsc` to the DSP worker.
//! - **Event path:** the typed event bus announces calibration metadata and
//!   lifecycle state. Complete sample matrices are not published on the bus.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aeryon_calibration::{CalibrationError, CalibrationPipeline};
use aeryon_csi::CsiFrame;
use aeryon_csi_replay::CsiFrameTx;
use aeryon_domain::{
    CalibrationFailed, CalibrationFailureCode, CalibrationServiceStopped, CalibrationStarted,
    CsiDataSource, CsiFrameCalibrated, Event, Timestamp,
};
use aeryon_dsp::CalibratedFrameTx;
use aeryon_events::EventBus;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::calibration_stats::{CalibrationStats, CalibrationWorkerState};
use crate::error::RuntimeError;
use crate::signal_store::SignalSnapshotStore;

/// Handles for a running calibration service.
pub struct CalibrationService {
    task: Option<JoinHandle<()>>,
    frame_tx: Option<CsiFrameTx>,
    worker_alive: Arc<AtomicBool>,
}

impl CalibrationService {
    /// Starts a single calibration worker bound to a validated pipeline.
    pub fn start(
        bus: EventBus,
        pipeline: CalibrationPipeline,
        stats: Arc<CalibrationStats>,
        queue_capacity: usize,
        calibrated_tx: Option<CalibratedFrameTx>,
        snapshot_store: Option<Arc<SignalSnapshotStore>>,
    ) -> Result<Self, RuntimeError> {
        if queue_capacity == 0 {
            return Err(RuntimeError::Config(
                crate::error::ConfigError::Calibration(CalibrationError::InvalidProfile {
                    message: "calibration.queue_capacity must be greater than zero".to_owned(),
                }),
            ));
        }

        let (frame_tx, mut frame_rx) = mpsc::channel::<Arc<CsiFrame>>(queue_capacity);
        let worker_alive = Arc::new(AtomicBool::new(true));
        let alive = Arc::clone(&worker_alive);
        let profile_id = pipeline.profile().id.clone();
        let profile_version = pipeline.profile().version;

        stats.reset_counters();
        stats.configure(true, Some(&profile_id), profile_version);
        stats.set_worker_state(CalibrationWorkerState::Running);

        let task = tokio::spawn(async move {
            let _ = bus.publish(Event::CalibrationStarted(CalibrationStarted {
                timestamp: now(),
                profile_id: profile_id.clone(),
                profile_version,
            }));

            while let Some(frame) = frame_rx.recv().await {
                let raw_frame_id = frame.frame_id();
                let sensor_id = frame.sensor_id();
                let sequence = frame.sequence();
                stats.record_submitted();

                match pipeline.calibrate(frame) {
                    Ok(calibrated) => {
                        let duration = calibrated.report().duration_ns;
                        stats.record_success(
                            calibrated.sequence(),
                            calibrated.calibrated_at().as_nanos(),
                            duration,
                        );
                        if let Some(warning) = calibrated.report().warning_summary() {
                            stats.set_last_warning(warning);
                        }

                        let source = match calibrated.source() {
                            aeryon_csi::CsiSourceKind::Replay => CsiDataSource::Replay,
                            aeryon_csi::CsiSourceKind::Live => CsiDataSource::Live,
                        };

                        let calibrated = Arc::new(calibrated);
                        if let Some(store) = &snapshot_store {
                            store.store_calibrated(Arc::clone(&calibrated));
                        }

                        let _ = bus.publish(Event::CsiFrameCalibrated(CsiFrameCalibrated {
                            raw_frame_id: calibrated.raw_frame_id(),
                            sensor_id: calibrated.sensor_id(),
                            sequence: calibrated.sequence(),
                            profile_id: calibrated.profile_id().to_owned(),
                            profile_version: calibrated.profile_version(),
                            stage_count: calibrated.report().stages.len() as u16,
                            calibration_duration_ns: duration,
                            receive_antennas: calibrated.receive_antennas(),
                            transmit_antennas: calibrated.transmit_antennas(),
                            subcarrier_count: calibrated.subcarrier_count() as u16,
                            source,
                            calibrated_at: calibrated.calibrated_at(),
                        }));

                        if let Some(tx) = &calibrated_tx {
                            // Await capacity — never silently drop calibrated frames.
                            if tx.send(Arc::clone(&calibrated)).await.is_err() {
                                tracing::warn!(
                                    "calibrated-frame DSP channel closed while calibration running"
                                );
                            }
                        }
                    }
                    Err(error) => {
                        stats.record_failure(error.to_string());
                        let _ = bus.publish(Event::CalibrationFailed(CalibrationFailed {
                            raw_frame_id: error.frame_id().or(Some(raw_frame_id)),
                            sensor_id: Some(sensor_id),
                            sequence: error.sequence().or(Some(sequence)),
                            timestamp: now(),
                            failed_stage: error.stage().map(|stage| stage.as_str().to_owned()),
                            code: map_failure_code(&error),
                            message: error.to_string(),
                        }));
                    }
                }
            }

            alive.store(false, Ordering::Relaxed);
            // Channel EOF is expected when the CSI producer drops its sender
            // after finite replay completion or graceful shutdown.
            stats.set_worker_state(CalibrationWorkerState::Stopped);

            let _ = bus.publish(Event::CalibrationServiceStopped(
                CalibrationServiceStopped { timestamp: now() },
            ));
        });

        Ok(Self {
            task: Some(task),
            frame_tx: Some(frame_tx),
            worker_alive,
        })
    }

    /// Returns the data-path sender for the CSI replay plugin.
    pub fn take_frame_tx(&mut self) -> Option<CsiFrameTx> {
        self.frame_tx.take()
    }

    /// Stops the worker without leaking the join handle.
    pub fn shutdown(&mut self) {
        drop(self.frame_tx.take());
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.worker_alive.store(false, Ordering::Relaxed);
    }

    /// Whether the worker task is still marked alive.
    pub fn is_alive(&self) -> bool {
        self.worker_alive.load(Ordering::Relaxed)
    }
}

impl Drop for CalibrationService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn map_failure_code(error: &CalibrationError) -> CalibrationFailureCode {
    match error {
        CalibrationError::InvalidProfile { .. } => CalibrationFailureCode::InvalidProfile,
        CalibrationError::UnsupportedStage { .. } => CalibrationFailureCode::UnsupportedStage,
        CalibrationError::MalformedFrame { .. } => CalibrationFailureCode::MalformedFrame,
        CalibrationError::NonFiniteSample { .. } => CalibrationFailureCode::NonFiniteSample,
        CalibrationError::InsufficientSubcarriers { .. } => {
            CalibrationFailureCode::InsufficientSubcarriers
        }
        CalibrationError::DegenerateRegression { .. } => {
            CalibrationFailureCode::DegenerateRegression
        }
        CalibrationError::ZeroEnergyLink { .. } => CalibrationFailureCode::ZeroEnergyLink,
        CalibrationError::StageFailure { .. } => CalibrationFailureCode::StageFailure,
        CalibrationError::OutputValidation { .. } => CalibrationFailureCode::OutputValidation,
        CalibrationError::PipelineUnavailable { .. } => CalibrationFailureCode::PipelineUnavailable,
    }
}

fn now() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    Timestamp::from_nanos(nanos)
}
