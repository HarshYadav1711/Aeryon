//! Runtime feature service: DSP results → feature vectors.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aeryon_domain::{
    Event, FeatureExtractionFailed, FeatureFailureCode, FeatureServiceIdle, FeatureServiceStarted,
    FeatureServiceStopped, FeatureVectorProduced, Timestamp,
};
use aeryon_dsp::DspWindowResult;
use aeryon_events::EventBus;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::errors::FeatureError;
use crate::extractor::extract_features;
use crate::profile::{FeatureProfile, FeaturesConfig};
use crate::stats::{FeatureStats, FeatureWorkerState};
use crate::vector::FeatureVector;

/// Bounded DSP-result input channel type.
pub type DspResultTx = mpsc::Sender<Arc<DspWindowResult>>;
/// Bounded DSP-result receiver type.
pub type DspResultRx = mpsc::Receiver<Arc<DspWindowResult>>;
/// Bounded feature-vector output channel type.
pub type FeatureVectorTx = mpsc::Sender<Arc<FeatureVector>>;

/// Optional sink notified when a feature vector is produced.
pub trait FeatureVectorSink: Send + Sync + 'static {
    /// Stores the latest successful feature vector.
    fn store_features(&self, vector: Arc<FeatureVector>);
}

/// Handles for a running feature service.
pub struct FeatureService {
    task: Option<JoinHandle<()>>,
    result_tx: Option<DspResultTx>,
    worker_alive: Arc<AtomicBool>,
}

impl FeatureService {
    /// Starts a single feature worker bound to a validated profile and config.
    pub fn start(
        bus: EventBus,
        config: FeaturesConfig,
        profile: FeatureProfile,
        stats: Arc<FeatureStats>,
        feature_sink: Option<Arc<dyn FeatureVectorSink>>,
        perception_tx: Option<FeatureVectorTx>,
    ) -> Result<Self, FeatureError> {
        config.validate()?;
        profile.validate()?;
        let schema = profile.schema()?;

        let queue_capacity = config.queue_capacity.max(1);
        let (result_tx, mut result_rx) = mpsc::channel::<Arc<DspWindowResult>>(queue_capacity);
        let worker_alive = Arc::new(AtomicBool::new(true));
        let alive = Arc::clone(&worker_alive);

        stats.reset_counters();
        stats.configure(
            true,
            Some(&profile.id),
            profile.version,
            Some(&schema.id),
            schema.version,
        );
        stats.set_worker_state(FeatureWorkerState::Running);

        let profile_id = profile.id.clone();
        let profile_version = profile.version;
        let schema_id = schema.id.clone();
        let schema_version = schema.version;
        let feature_count = schema.length() as u32;

        let task = tokio::spawn(async move {
            let _ = bus.publish(Event::FeatureServiceStarted(FeatureServiceStarted {
                timestamp: now(),
                profile_id: profile_id.clone(),
                profile_version,
                schema_id: schema_id.clone(),
                schema_version,
            }));

            let mut received_any = false;

            while let Some(dsp_result) = result_rx.recv().await {
                received_any = true;
                stats.record_dsp_received();

                match extract_features(&dsp_result, &profile) {
                    Ok((vector, report)) => {
                        if let Some(warning) = report.warnings.first() {
                            stats.set_last_warning(warning.clone());
                        }
                        stats.record_success(
                            vector.feature_vector_id,
                            vector.first_sequence,
                            vector.last_sequence,
                            vector.processing_duration_ns,
                        );

                        let vector = Arc::new(vector);
                        if let Some(sink) = &feature_sink {
                            sink.store_features(Arc::clone(&vector));
                        }
                        if let Some(tx) = &perception_tx {
                            if tx.send(Arc::clone(&vector)).await.is_err() {
                                tracing::warn!(
                                    "perception channel closed while forwarding feature vector"
                                );
                            }
                        }

                        let _ = bus.publish(Event::FeatureVectorProduced(FeatureVectorProduced {
                            feature_vector_id: vector.feature_vector_id,
                            sensor_id: vector.sensor_id,
                            window_id: vector.window_id,
                            first_sequence: vector.first_sequence,
                            last_sequence: vector.last_sequence,
                            schema_id: vector.feature_schema_id.clone(),
                            schema_version: vector.feature_schema_version,
                            profile_id: vector.feature_profile_id.clone(),
                            profile_version: vector.feature_profile_version,
                            feature_count,
                            link_count: vector.link_count() as u32,
                            processing_duration_ns: vector.processing_duration_ns,
                            extracted_at: vector.extracted_at,
                        }));
                    }
                    Err(error) => {
                        stats.record_failure(error.to_string());
                        let _ =
                            bus.publish(Event::FeatureExtractionFailed(FeatureExtractionFailed {
                                window_id: Some(dsp_result.window_id),
                                sensor_id: Some(dsp_result.sensor_id),
                                first_sequence: Some(dsp_result.first_sequence),
                                last_sequence: Some(dsp_result.last_sequence),
                                timestamp: now(),
                                code: map_failure_code(&error),
                                message: error.to_string(),
                            }));
                    }
                }
            }

            alive.store(false, Ordering::Relaxed);
            let completed = received_any;
            stats.set_worker_state(if completed {
                FeatureWorkerState::Completed
            } else {
                FeatureWorkerState::Idle
            });
            let _ = bus.publish(Event::FeatureServiceIdle(FeatureServiceIdle {
                timestamp: now(),
                completed,
            }));
            let _ = bus.publish(Event::FeatureServiceStopped(FeatureServiceStopped {
                timestamp: now(),
            }));
        });

        Ok(Self {
            task: Some(task),
            result_tx: Some(result_tx),
            worker_alive,
        })
    }

    /// Returns the data-path sender for the DSP worker.
    pub fn take_result_tx(&mut self) -> Option<DspResultTx> {
        self.result_tx.take()
    }

    /// Stops the worker without leaking the join handle.
    pub fn shutdown(&mut self) {
        drop(self.result_tx.take());
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

impl Drop for FeatureService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn map_failure_code(error: &FeatureError) -> FeatureFailureCode {
    match error {
        FeatureError::IncompatibleDspProfile { .. } => FeatureFailureCode::IncompatibleDspProfile,
        FeatureError::MissingMotionEnergy { .. } => FeatureFailureCode::MissingMotionEnergy,
        FeatureError::MissingSpectrum { .. } => FeatureFailureCode::MissingSpectrum,
        FeatureError::MismatchedLinkData { .. } => FeatureFailureCode::MismatchedLinkData,
        FeatureError::EmptySignal { .. } => FeatureFailureCode::EmptySignal,
        FeatureError::NonFiniteInput { .. } => FeatureFailureCode::NonFinite,
        FeatureError::InvalidPower { .. } => FeatureFailureCode::InvalidPower,
        FeatureError::ZeroTotalPower { .. } => FeatureFailureCode::ZeroTotalPower,
        FeatureError::InvalidProfile { .. } => FeatureFailureCode::InvalidProfile,
        FeatureError::SchemaMismatch { .. } => FeatureFailureCode::SchemaMismatch,
        FeatureError::OutputValidation { .. } => FeatureFailureCode::OutputValidation,
        FeatureError::ServiceFailure { .. } => FeatureFailureCode::ServiceFailure,
    }
}

fn now() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    Timestamp::from_nanos(nanos)
}
