//! Runtime DSP service: calibrated frames → windows → spectral results.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aeryon_calibration::CalibratedCsiFrame;
use aeryon_domain::{
    CsiWindowAssembled, DspFailureCode, DspProcessingFailed, DspServiceIdle, DspServiceStarted,
    DspServiceStopped, DspWindowProcessed, Event, Timestamp,
};
use aeryon_events::EventBus;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::assembler::WindowAssembler;
use crate::backend::{DspKernelBackend, create_backend};
use crate::errors::DspError;
use crate::profile::{DspConfig, DspProfile};
use crate::report::process_window;
use crate::result::DspWindowResult;
use crate::stats::{DspStats, DspWorkerState};

/// Bounded calibrated-frame input channel type.
pub type CalibratedFrameTx = mpsc::Sender<Arc<CalibratedCsiFrame>>;
/// Bounded calibrated-frame receiver type.
pub type CalibratedFrameRx = mpsc::Receiver<Arc<CalibratedCsiFrame>>;

/// Optional sink notified when a DSP result is produced.
pub trait DspResultSink: Send + Sync + 'static {
    /// Stores the latest successful DSP result.
    fn store_result(&self, result: Arc<DspWindowResult>);
}

/// Bounded channel for forwarding DSP results to a downstream consumer (features).
pub type DspResultForwardTx = mpsc::Sender<Arc<DspWindowResult>>;

/// Handles for a running DSP service.
pub struct DspService {
    task: Option<JoinHandle<()>>,
    frame_tx: Option<CalibratedFrameTx>,
    worker_alive: Arc<AtomicBool>,
}

impl DspService {
    /// Starts a single DSP worker bound to a validated profile and config.
    pub fn start(
        bus: EventBus,
        config: DspConfig,
        profile: DspProfile,
        stats: Arc<DspStats>,
        result_sink: Option<Arc<dyn DspResultSink>>,
        result_forward_tx: Option<DspResultForwardTx>,
    ) -> Result<Self, DspError> {
        config.validate()?;
        profile.validate()?;
        let backend = create_backend(config.backend)?;
        let identity = backend.identity();

        let queue_capacity = config.queue_capacity.max(1);
        let (frame_tx, mut frame_rx) = mpsc::channel::<Arc<CalibratedCsiFrame>>(queue_capacity);
        let worker_alive = Arc::new(AtomicBool::new(true));
        let alive = Arc::clone(&worker_alive);
        let assembler_config = config.assembler_config();
        let mut assembler = WindowAssembler::try_new(assembler_config)?;

        stats.reset_counters();
        stats.configure(
            true,
            Some(&profile.id),
            profile.version,
            config.window_size_frames,
            config.hop_size_frames,
        );
        stats.set_backend_identity(
            identity.kind.as_str(),
            &identity.implementation_version,
            identity.abi_version,
            true,
            "ok",
            None,
        );
        stats.set_worker_state(DspWorkerState::Running);

        let profile_id = profile.id.clone();
        let profile_version = profile.version;
        let window_size_frames = config.window_size_frames as u32;
        let hop_size_frames = config.hop_size_frames as u32;
        let backend_id = identity.kind.as_str().to_owned();
        let backend_version = identity.implementation_version.clone();
        let backend_abi_version = identity.abi_version;
        let backend: Arc<dyn DspKernelBackend> = backend;

        let task = tokio::spawn(async move {
            let _ = bus.publish(Event::DspServiceStarted(DspServiceStarted {
                timestamp: now(),
                profile_id: profile_id.clone(),
                profile_version,
                window_size_frames,
                hop_size_frames,
                backend_id,
                backend_version,
                backend_abi_version,
            }));

            let mut received_any = false;

            while let Some(frame) = frame_rx.recv().await {
                received_any = true;
                stats.record_frame_received();

                match assembler.push(frame) {
                    Ok(None) => {}
                    Ok(Some(window)) => {
                        let _ = bus.publish(Event::CsiWindowAssembled(CsiWindowAssembled {
                            window_id: window.window_id(),
                            sensor_id: window.sensor_id(),
                            first_sequence: window.first_sequence(),
                            last_sequence: window.last_sequence(),
                            frame_count: window.frame_count() as u32,
                            timestamp: now(),
                        }));

                        match process_window(&window, &profile, backend.as_ref()) {
                            Ok(result) => {
                                if let Some(warning) = result.warnings.first() {
                                    stats.set_last_warning(warning.clone());
                                }
                                stats.record_window_success(
                                    result.first_sequence,
                                    result.last_sequence,
                                    result.processed_at.as_nanos(),
                                    result.processing_duration_ns,
                                    result.sampling.effective_sample_rate_hz,
                                    result.sampling.timestamp_jitter,
                                    result.dominant_non_dc_hz(),
                                );

                                let result = Arc::new(result);
                                if let Some(sink) = &result_sink {
                                    sink.store_result(Arc::clone(&result));
                                }
                                if let Some(tx) = &result_forward_tx {
                                    if tx.send(Arc::clone(&result)).await.is_err() {
                                        tracing::warn!(
                                            "downstream DSP result channel closed while forwarding"
                                        );
                                    }
                                }

                                let _ =
                                    bus.publish(Event::DspWindowProcessed(DspWindowProcessed {
                                        window_id: result.window_id,
                                        sensor_id: result.sensor_id,
                                        first_sequence: result.first_sequence,
                                        last_sequence: result.last_sequence,
                                        frame_count: result.frame_count as u32,
                                        profile_id: result.dsp_profile_id.clone(),
                                        profile_version: result.dsp_profile_version,
                                        processing_duration_ns: result.processing_duration_ns,
                                        effective_sample_rate_hz: result
                                            .sampling
                                            .effective_sample_rate_hz,
                                        timestamp_jitter: result.sampling.timestamp_jitter,
                                        dominant_non_dc_hz: result.dominant_non_dc_hz(),
                                        processed_at: result.processed_at,
                                    }));
                            }
                            Err(error) => {
                                stats.record_window_failure(error.to_string());
                                let _ =
                                    bus.publish(Event::DspProcessingFailed(DspProcessingFailed {
                                        window_id: Some(window.window_id()),
                                        sensor_id: Some(window.sensor_id()),
                                        first_sequence: Some(window.first_sequence()),
                                        last_sequence: Some(window.last_sequence()),
                                        timestamp: now(),
                                        code: map_failure_code(&error),
                                        message: error.to_string(),
                                    }));
                            }
                        }
                    }
                    Err(error) => {
                        stats.record_window_failure(error.to_string());
                        let _ = bus.publish(Event::DspProcessingFailed(DspProcessingFailed {
                            window_id: None,
                            sensor_id: error.sensor_id(),
                            first_sequence: error.sequence(),
                            last_sequence: error.sequence(),
                            timestamp: now(),
                            code: map_failure_code(&error),
                            message: error.to_string(),
                        }));
                    }
                }
            }

            alive.store(false, Ordering::Relaxed);
            // Finite calibrated-frame EOF is expected after CSI replay completion.
            let completed = received_any;
            stats.set_worker_state(if completed {
                DspWorkerState::Completed
            } else {
                DspWorkerState::Idle
            });
            let _ = bus.publish(Event::DspServiceIdle(DspServiceIdle {
                timestamp: now(),
                completed,
            }));
            let _ = bus.publish(Event::DspServiceStopped(DspServiceStopped {
                timestamp: now(),
            }));
        });

        Ok(Self {
            task: Some(task),
            frame_tx: Some(frame_tx),
            worker_alive,
        })
    }

    /// Returns the data-path sender for the calibration worker.
    pub fn take_frame_tx(&mut self) -> Option<CalibratedFrameTx> {
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

impl Drop for DspService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn map_failure_code(error: &DspError) -> DspFailureCode {
    match error.code() {
        crate::errors::DspFailureCode::InvalidConfig => DspFailureCode::InvalidConfig,
        crate::errors::DspFailureCode::BackendUnavailable => DspFailureCode::BackendUnavailable,
        crate::errors::DspFailureCode::NativeKernel => DspFailureCode::NativeKernel,
        crate::errors::DspFailureCode::InvalidWindow => DspFailureCode::InvalidWindow,
        crate::errors::DspFailureCode::SensorMismatch => DspFailureCode::SensorMismatch,
        crate::errors::DspFailureCode::GeometryMismatch => DspFailureCode::GeometryMismatch,
        crate::errors::DspFailureCode::CalibrationProfileMismatch => {
            DspFailureCode::CalibrationProfileMismatch
        }
        crate::errors::DspFailureCode::NonMonotonicSequence => DspFailureCode::NonMonotonicSequence,
        crate::errors::DspFailureCode::SequenceGap => DspFailureCode::SequenceGap,
        crate::errors::DspFailureCode::NonMonotonicTimestamp => {
            DspFailureCode::NonMonotonicTimestamp
        }
        crate::errors::DspFailureCode::ExcessiveJitter => DspFailureCode::ExcessiveJitter,
        crate::errors::DspFailureCode::MotionEnergy => DspFailureCode::MotionEnergy,
        crate::errors::DspFailureCode::Spectral => DspFailureCode::Spectral,
        crate::errors::DspFailureCode::InsufficientLength => DspFailureCode::InsufficientLength,
        crate::errors::DspFailureCode::InvalidSampleRate => DspFailureCode::InvalidSampleRate,
        crate::errors::DspFailureCode::NonFinite => DspFailureCode::NonFinite,
        crate::errors::DspFailureCode::OutputValidation => DspFailureCode::OutputValidation,
        crate::errors::DspFailureCode::WorkerExited => DspFailureCode::WorkerExited,
    }
}

fn now() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    Timestamp::from_nanos(nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeryon_calibration::{CalibrationPipeline, baseline_csi_v1};
    use aeryon_csi::{ComplexSample, CsiFrame, CsiRadioMetadata, CsiSourceKind};
    use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId};
    use tokio::time::{Duration, timeout};

    struct MemorySink(std::sync::Mutex<Option<Arc<DspWindowResult>>>);

    impl DspResultSink for MemorySink {
        fn store_result(&self, result: Arc<DspWindowResult>) {
            *self.0.lock().expect("lock") = Some(result);
        }
    }

    fn calibrated(sequence: u64) -> Arc<CalibratedCsiFrame> {
        let samples = vec![
            ComplexSample::new(1.0 + sequence as f32 * 0.01, 0.2),
            ComplexSample::new(0.8, 0.1 + sequence as f32 * 0.02),
            ComplexSample::new(1.1, -0.2),
            ComplexSample::new(0.9, 0.3),
        ];
        let nanos = sequence * 100_000_000;
        let metadata = FrameMetadata {
            frame_id: FrameId::new(sequence + 1),
            sensor_id: SensorId::new(2),
            timestamp: Timestamp::from_nanos(nanos),
            sequence,
            mission_id: None,
            metadata: Metadata::new(),
        };
        let raw = CsiFrame::try_new(
            metadata,
            Timestamp::from_nanos(nanos),
            None,
            None,
            1,
            1,
            vec![0, 1, 2, 3],
            samples,
            CsiSourceKind::Replay,
            CsiRadioMetadata::default(),
        )
        .expect("raw");
        let pipeline = CalibrationPipeline::try_new(baseline_csi_v1()).expect("pipeline");
        Arc::new(pipeline.calibrate(Arc::new(raw)).expect("calibrated"))
    }

    #[tokio::test]
    async fn service_emits_ordered_windows_and_completes() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let stats = DspStats::new().shared();
        let sink = Arc::new(MemorySink(std::sync::Mutex::new(None)));
        let config = DspConfig {
            enabled: true,
            window_size_frames: 8,
            hop_size_frames: 4,
            queue_capacity: 32,
            ..DspConfig::default()
        };
        let profile = config.resolve_profile().expect("profile");
        let mut service = DspService::start(
            bus,
            config,
            profile,
            Arc::clone(&stats),
            Some(sink.clone()),
            None,
        )
        .expect("start");
        let tx = service.take_frame_tx().expect("tx");

        for sequence in 0..12 {
            tx.send(calibrated(sequence)).await.expect("send");
        }
        drop(tx);

        let processed = timeout(Duration::from_secs(2), async {
            let mut count = 0_u32;
            loop {
                match rx.recv().await {
                    Ok(Event::DspWindowProcessed(_)) => {
                        count += 1;
                        if count >= 2 {
                            break count;
                        }
                    }
                    Ok(Event::DspServiceIdle(event)) => {
                        assert!(event.completed);
                        if count >= 1 {
                            break count;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break count,
                }
            }
        })
        .await
        .expect("timeout");
        assert!(processed >= 1);
        assert!(stats.windows_emitted() >= 1);
        assert!(sink.0.lock().expect("lock").is_some());

        timeout(Duration::from_secs(1), async {
            while stats.worker_state() == DspWorkerState::Running {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("complete");
        assert!(matches!(
            stats.worker_state(),
            DspWorkerState::Completed | DspWorkerState::Idle
        ));
        service.shutdown();
        assert!(!service.is_alive());
    }

    #[test]
    fn disabled_config_validate_ok_without_worker() {
        let config = DspConfig {
            enabled: false,
            ..DspConfig::default()
        };
        config.validate().expect("disabled ok");
    }

    #[test]
    fn invalid_config_rejected() {
        let config = DspConfig {
            enabled: true,
            window_size_frames: 1,
            ..DspConfig::default()
        };
        assert!(config.validate().is_err());
    }
}
