//! Runtime perception service: feature vectors → channel-change observations.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use aeryon_domain::{
    ChannelChangeObserved, Event, ObservationFailed, ObservationFailureCode, PerceptionServiceIdle,
    PerceptionServiceStarted, PerceptionServiceStopped, Timestamp,
};
use aeryon_events::EventBus;
use aeryon_features::{FeatureVector, FeatureVectorTx};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::channel_change::observe_channel_change;
use crate::errors::PerceptionError;
use crate::observation::ChannelChangeObservation;
use crate::profile::{ChannelChangeProfile, PerceptionConfig};
use crate::stats::{PerceptionStats, PerceptionWorkerState};

/// Bounded feature-vector receiver type.
pub type FeatureVectorRx = mpsc::Receiver<Arc<FeatureVector>>;

/// Optional sink notified when an observation is produced.
pub trait ObservationSink: Send + Sync + 'static {
    /// Stores the latest successful channel-change observation.
    fn store_observation(&self, observation: Arc<ChannelChangeObservation>);
}

/// Handles for a running perception service.
pub struct PerceptionService {
    task: Option<JoinHandle<()>>,
    feature_tx: Option<FeatureVectorTx>,
    worker_alive: Arc<AtomicBool>,
}

impl PerceptionService {
    /// Starts a single perception worker bound to a validated profile and config.
    pub fn start(
        bus: EventBus,
        config: PerceptionConfig,
        profile: ChannelChangeProfile,
        stats: Arc<PerceptionStats>,
        observation_sink: Option<Arc<dyn ObservationSink>>,
    ) -> Result<Self, PerceptionError> {
        config.validate()?;
        profile.validate()?;

        let queue_capacity = config.queue_capacity.max(1);
        let (feature_tx, mut feature_rx) = mpsc::channel::<Arc<FeatureVector>>(queue_capacity);
        let worker_alive = Arc::new(AtomicBool::new(true));
        let alive = Arc::clone(&worker_alive);

        stats.reset_counters();
        stats.configure(true, Some(&profile.id), profile.version);
        stats.set_worker_state(PerceptionWorkerState::Running);

        let profile_id = profile.id.clone();
        let profile_version = profile.version;

        let task = tokio::spawn(async move {
            let _ = bus.publish(Event::PerceptionServiceStarted(PerceptionServiceStarted {
                timestamp: now(),
                profile_id: profile_id.clone(),
                profile_version,
            }));

            let mut received_any = false;

            while let Some(vector) = feature_rx.recv().await {
                received_any = true;
                stats.record_feature_received();
                let started = Instant::now();

                match observe_channel_change(&vector, &profile) {
                    Ok(observation) => {
                        let duration_ns =
                            u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
                        if let Some(warning) = observation.warnings.first() {
                            stats.set_last_warning(warning.clone());
                        }
                        stats.record_success(
                            observation.observation_id,
                            observation.state.as_str(),
                            observation.activity_score,
                            observation.evidence.threshold_margin,
                            duration_ns,
                        );

                        let observation = Arc::new(observation);
                        if let Some(sink) = &observation_sink {
                            sink.store_observation(Arc::clone(&observation));
                        }

                        let _ = bus.publish(Event::ChannelChangeObserved(ChannelChangeObserved {
                            observation_id: observation.observation_id,
                            sensor_id: observation.sensor_id,
                            feature_vector_id: observation.feature_vector_id,
                            first_sequence: observation.first_sequence,
                            last_sequence: observation.last_sequence,
                            state: observation.state.as_str().to_owned(),
                            activity_score: observation.activity_score,
                            threshold_margin: observation.evidence.threshold_margin,
                            profile_id: observation.threshold_profile_id.clone(),
                            profile_version: observation.threshold_profile_version,
                            warning_count: observation.warnings.len() as u32,
                            created_at: observation.created_at,
                        }));
                    }
                    Err(error) => {
                        stats.record_failure(error.to_string());
                        let _ = bus.publish(Event::ObservationFailed(ObservationFailed {
                            feature_vector_id: Some(vector.feature_vector_id),
                            sensor_id: Some(vector.sensor_id),
                            first_sequence: Some(vector.first_sequence),
                            last_sequence: Some(vector.last_sequence),
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
                PerceptionWorkerState::Completed
            } else {
                PerceptionWorkerState::Idle
            });
            let _ = bus.publish(Event::PerceptionServiceIdle(PerceptionServiceIdle {
                timestamp: now(),
                completed,
            }));
            let _ = bus.publish(Event::PerceptionServiceStopped(PerceptionServiceStopped {
                timestamp: now(),
            }));
        });

        Ok(Self {
            task: Some(task),
            feature_tx: Some(feature_tx),
            worker_alive,
        })
    }

    /// Returns the data-path sender for the feature worker.
    pub fn take_feature_tx(&mut self) -> Option<FeatureVectorTx> {
        self.feature_tx.take()
    }

    /// Stops the worker without leaking the join handle.
    pub fn shutdown(&mut self) {
        drop(self.feature_tx.take());
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

impl Drop for PerceptionService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn map_failure_code(error: &PerceptionError) -> ObservationFailureCode {
    match error {
        PerceptionError::IncompatibleFeatureSchema { .. } => {
            ObservationFailureCode::IncompatibleFeatureSchema
        }
        PerceptionError::MissingFeatures { .. } => ObservationFailureCode::MissingFeatures,
        PerceptionError::InvalidProfile { .. } => ObservationFailureCode::InvalidProfile,
        PerceptionError::NonFinite { .. } => ObservationFailureCode::NonFinite,
        PerceptionError::OutputValidation { .. } => ObservationFailureCode::OutputValidation,
        PerceptionError::ServiceFailure { .. } => ObservationFailureCode::ServiceFailure,
    }
}

fn now() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    Timestamp::from_nanos(nanos)
}
