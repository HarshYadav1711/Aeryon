//! Strongly typed domain events.

use std::sync::Arc;

use crate::ids::{EntityId, FrameId, MissionId, SensorId};
use crate::observation::Observation;
use crate::pipeline::PipelineStageId;
use crate::time::Timestamp;
use crate::world::{WorldEntity, WorldRelationship};

/// Origin classification for CSI metadata events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsiDataSource {
    /// Deterministic development fixture replay (not live RF).
    Replay,
    /// Live hardware capture.
    Live,
}

impl CsiDataSource {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Replay => "csi_replay",
            Self::Live => "csi_live",
        }
    }
}

/// CSI replay plugin started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsiReplayStarted {
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Start timestamp.
    pub timestamp: Timestamp,
}

/// Lightweight CSI frame metadata published on the event bus.
///
/// The complex sample matrix is intentionally omitted. Optional shared ownership
/// of a modality-agnostic payload token allows producers to retain frames without
/// forcing every subscriber to clone sample data.
#[derive(Debug, Clone, PartialEq)]
pub struct CsiFrameReceived {
    /// Frame identifier.
    pub frame_id: FrameId,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Monotonic sequence number.
    pub sequence: u64,
    /// Capture / acquisition timestamp.
    pub capture_timestamp: Timestamp,
    /// Receive or replay timestamp.
    pub receive_timestamp: Timestamp,
    /// Receive antenna count.
    pub receive_antennas: u16,
    /// Transmit antenna count.
    pub transmit_antennas: u16,
    /// Number of subcarriers.
    pub subcarrier_count: u16,
    /// Optional center frequency in hertz.
    pub center_frequency_hz: Option<f64>,
    /// Optional channel bandwidth in hertz.
    pub bandwidth_hz: Option<f64>,
    /// Frame origin classification.
    pub source: CsiDataSource,
    /// Optional shared handle retained by producers (for example an `Arc` token).
    pub frame_token: Option<Arc<()>>,
}

/// CSI fixture replay completed a finite pass without failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsiReplayCompleted {
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Completion timestamp.
    pub timestamp: Timestamp,
    /// Number of frames accepted during the completed pass.
    pub frames_accepted: u64,
}

/// CSI replay plugin stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsiReplayStopped {
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Stop timestamp.
    pub timestamp: Timestamp,
}

/// Classification of a CSI replay failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsiReplayFailureKind {
    /// Fixture could not be opened or parsed.
    FixtureError,
    /// A malformed frame was encountered.
    MalformedFrame,
    /// Publishing a CSI event failed.
    PublishFailed,
    /// The producer task exited unexpectedly.
    ProducerExited,
}

/// CSI replay entered a failure state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsiReplayFailed {
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Failure timestamp.
    pub timestamp: Timestamp,
    /// Failure classification.
    pub kind: CsiReplayFailureKind,
}

/// Calibration service started applying a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationStarted {
    /// Start timestamp.
    pub timestamp: Timestamp,
    /// Active profile identity.
    pub profile_id: String,
    /// Active profile version.
    pub profile_version: u32,
}

/// Metadata announcing a successfully calibrated CSI frame.
///
/// Complete calibrated sample matrices are intentionally omitted from the event
/// bus. Frame data travels on the dedicated calibration data path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsiFrameCalibrated {
    /// Raw frame identifier.
    pub raw_frame_id: FrameId,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Monotonic sequence number.
    pub sequence: u64,
    /// Calibration profile identity.
    pub profile_id: String,
    /// Calibration profile version.
    pub profile_version: u32,
    /// Number of stages executed.
    pub stage_count: u16,
    /// Calibration duration in nanoseconds.
    pub calibration_duration_ns: u64,
    /// Receive antenna count.
    pub receive_antennas: u16,
    /// Transmit antenna count.
    pub transmit_antennas: u16,
    /// Subcarrier count.
    pub subcarrier_count: u16,
    /// Frame origin classification (CSI replay development data).
    pub source: CsiDataSource,
    /// Pipeline completion timestamp.
    pub calibrated_at: Timestamp,
}

/// Machine-readable calibration failure codes for API surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalibrationFailureCode {
    /// Profile construction or validation failed.
    InvalidProfile,
    /// Unsupported stage requested.
    UnsupportedStage,
    /// Malformed frame dimensions.
    MalformedFrame,
    /// Non-finite sample encountered.
    NonFiniteSample,
    /// Insufficient subcarrier information.
    InsufficientSubcarriers,
    /// Degenerate linear regression.
    DegenerateRegression,
    /// Zero-energy antenna link.
    ZeroEnergyLink,
    /// Generic stage failure.
    StageFailure,
    /// Output validation failure.
    OutputValidation,
    /// Pipeline unavailable or disabled unexpectedly.
    PipelineUnavailable,
    /// Calibration worker exited unexpectedly.
    WorkerExited,
}

impl CalibrationFailureCode {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidProfile => "invalid_profile",
            Self::UnsupportedStage => "unsupported_stage",
            Self::MalformedFrame => "malformed_frame",
            Self::NonFiniteSample => "non_finite_sample",
            Self::InsufficientSubcarriers => "insufficient_subcarriers",
            Self::DegenerateRegression => "degenerate_regression",
            Self::ZeroEnergyLink => "zero_energy_link",
            Self::StageFailure => "stage_failure",
            Self::OutputValidation => "output_validation",
            Self::PipelineUnavailable => "pipeline_unavailable",
            Self::WorkerExited => "worker_exited",
        }
    }
}

/// Calibration failed for a frame or service-level fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationFailed {
    /// Raw frame identifier when available.
    pub raw_frame_id: Option<FrameId>,
    /// Source sensor identifier when available.
    pub sensor_id: Option<SensorId>,
    /// Sequence when available.
    pub sequence: Option<u64>,
    /// Failure timestamp.
    pub timestamp: Timestamp,
    /// Failed stage label when applicable (for example `phase_unwrap`).
    pub failed_stage: Option<String>,
    /// Typed failure code.
    pub code: CalibrationFailureCode,
    /// Concise operator-safe message (no Rust debug dumps).
    pub message: String,
}

/// Calibration service stopped cleanly or after cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationServiceStopped {
    /// Stop timestamp.
    pub timestamp: Timestamp,
}

/// DSP service started applying a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DspServiceStarted {
    /// Start timestamp.
    pub timestamp: Timestamp,
    /// Active DSP profile identity.
    pub profile_id: String,
    /// Active DSP profile version.
    pub profile_version: u32,
    /// Temporal window size in frames.
    pub window_size_frames: u32,
    /// Hop size in frames.
    pub hop_size_frames: u32,
    /// Selected kernel backend identifier (`rust` or `cpp`).
    pub backend_id: String,
    /// Backend implementation version.
    pub backend_version: String,
    /// Native ABI version when the C++ backend is active.
    pub backend_abi_version: Option<u32>,
}

/// A temporal CSI window was assembled (metadata only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsiWindowAssembled {
    /// Window identity.
    pub window_id: u64,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// Frame count in the window.
    pub frame_count: u32,
    /// Assembly timestamp.
    pub timestamp: Timestamp,
}

/// A DSP window was processed successfully (metadata only; no spectra arrays).
#[derive(Debug, Clone, PartialEq)]
pub struct DspWindowProcessed {
    /// Window identity.
    pub window_id: u64,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// Frame count in the window.
    pub frame_count: u32,
    /// Active DSP profile identity.
    pub profile_id: String,
    /// Active DSP profile version.
    pub profile_version: u32,
    /// Processing duration in nanoseconds.
    pub processing_duration_ns: u64,
    /// Effective sample rate derived from capture timestamps.
    pub effective_sample_rate_hz: f64,
    /// Timestamp jitter metric for the window.
    pub timestamp_jitter: f64,
    /// Dominant non-DC frequency when available.
    pub dominant_non_dc_hz: Option<f64>,
    /// Processing completion timestamp.
    pub processed_at: Timestamp,
}

/// Machine-readable DSP failure codes for API surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DspFailureCode {
    /// Profile or configuration validation failed.
    InvalidConfig,
    /// Requested DSP kernel backend is unavailable.
    BackendUnavailable,
    /// Native DSP kernel status mapping failure.
    NativeKernel,
    /// Window geometry or temporal invariants failed.
    InvalidWindow,
    /// Sensor mismatch across frames.
    SensorMismatch,
    /// Antenna or subcarrier geometry mismatch.
    GeometryMismatch,
    /// Calibration profile identity or version mismatch.
    CalibrationProfileMismatch,
    /// Sequence numbers are not strictly increasing.
    NonMonotonicSequence,
    /// Sequence gap exceeds configured tolerance.
    SequenceGap,
    /// Capture timestamps are not monotonic.
    NonMonotonicTimestamp,
    /// Timestamp jitter exceeds spectral tolerance.
    ExcessiveJitter,
    /// Motion-energy proxy computation failed.
    MotionEnergy,
    /// Spectral analysis rejected the input.
    Spectral,
    /// Insufficient samples for spectral analysis.
    InsufficientLength,
    /// Effective sample rate is invalid.
    InvalidSampleRate,
    /// Non-finite intermediate or output values.
    NonFinite,
    /// Output validation failed.
    OutputValidation,
    /// DSP worker exited unexpectedly.
    WorkerExited,
}

impl DspFailureCode {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidConfig => "invalid_config",
            Self::BackendUnavailable => "backend_unavailable",
            Self::NativeKernel => "native_kernel",
            Self::InvalidWindow => "invalid_window",
            Self::SensorMismatch => "sensor_mismatch",
            Self::GeometryMismatch => "geometry_mismatch",
            Self::CalibrationProfileMismatch => "calibration_profile_mismatch",
            Self::NonMonotonicSequence => "non_monotonic_sequence",
            Self::SequenceGap => "sequence_gap",
            Self::NonMonotonicTimestamp => "non_monotonic_timestamp",
            Self::ExcessiveJitter => "excessive_jitter",
            Self::MotionEnergy => "motion_energy",
            Self::Spectral => "spectral",
            Self::InsufficientLength => "insufficient_length",
            Self::InvalidSampleRate => "invalid_sample_rate",
            Self::NonFinite => "non_finite",
            Self::OutputValidation => "output_validation",
            Self::WorkerExited => "worker_exited",
        }
    }
}

/// DSP processing failed for a window or service-level fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DspProcessingFailed {
    /// Window identity when available.
    pub window_id: Option<u64>,
    /// Source sensor identifier when available.
    pub sensor_id: Option<SensorId>,
    /// Inclusive first sequence when available.
    pub first_sequence: Option<u64>,
    /// Inclusive last sequence when available.
    pub last_sequence: Option<u64>,
    /// Failure timestamp.
    pub timestamp: Timestamp,
    /// Typed failure code.
    pub code: DspFailureCode,
    /// Concise operator-safe message.
    pub message: String,
}

/// DSP service became idle or completed after finite input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DspServiceIdle {
    /// Idle / completed timestamp.
    pub timestamp: Timestamp,
    /// Whether finite input completed (`true`) versus idle without windows.
    pub completed: bool,
}

/// DSP service stopped cleanly or after cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DspServiceStopped {
    /// Stop timestamp.
    pub timestamp: Timestamp,
}

/// Feature extraction service started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureServiceStarted {
    /// Start timestamp.
    pub timestamp: Timestamp,
    /// Active feature profile identity.
    pub profile_id: String,
    /// Active feature profile version.
    pub profile_version: u32,
    /// Active feature schema identity.
    pub schema_id: String,
    /// Active feature schema version.
    pub schema_version: u32,
}

/// A feature vector was produced (metadata only; no numerical arrays).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureVectorProduced {
    /// Feature-vector identity.
    pub feature_vector_id: u64,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Source DSP window identity.
    pub window_id: u64,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// Feature schema identity.
    pub schema_id: String,
    /// Feature schema version.
    pub schema_version: u32,
    /// Feature profile identity.
    pub profile_id: String,
    /// Feature profile version.
    pub profile_version: u32,
    /// Number of aggregate features.
    pub feature_count: u32,
    /// Number of antenna links.
    pub link_count: u32,
    /// Processing duration in nanoseconds.
    pub processing_duration_ns: u64,
    /// Extraction completion timestamp.
    pub extracted_at: Timestamp,
}

/// Machine-readable feature extraction failure codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureFailureCode {
    /// DSP profile incompatible with the feature profile.
    IncompatibleDspProfile,
    /// Motion-energy series missing.
    MissingMotionEnergy,
    /// Spectrum missing.
    MissingSpectrum,
    /// Per-link data mismatch.
    MismatchedLinkData,
    /// Empty signal input.
    EmptySignal,
    /// Non-finite values.
    NonFinite,
    /// Invalid spectral power.
    InvalidPower,
    /// Zero total power where unsupported.
    ZeroTotalPower,
    /// Invalid feature profile.
    InvalidProfile,
    /// Schema mismatch.
    SchemaMismatch,
    /// Output validation failure.
    OutputValidation,
    /// Service-level failure.
    ServiceFailure,
}

impl FeatureFailureCode {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IncompatibleDspProfile => "incompatible_dsp_profile",
            Self::MissingMotionEnergy => "missing_motion_energy",
            Self::MissingSpectrum => "missing_spectrum",
            Self::MismatchedLinkData => "mismatched_link_data",
            Self::EmptySignal => "empty_signal",
            Self::NonFinite => "non_finite",
            Self::InvalidPower => "invalid_power",
            Self::ZeroTotalPower => "zero_total_power",
            Self::InvalidProfile => "invalid_profile",
            Self::SchemaMismatch => "schema_mismatch",
            Self::OutputValidation => "output_validation",
            Self::ServiceFailure => "service_failure",
        }
    }
}

/// Feature extraction failed for a window or service-level fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureExtractionFailed {
    /// Window identity when available.
    pub window_id: Option<u64>,
    /// Source sensor identifier when available.
    pub sensor_id: Option<SensorId>,
    /// Inclusive first sequence when available.
    pub first_sequence: Option<u64>,
    /// Inclusive last sequence when available.
    pub last_sequence: Option<u64>,
    /// Failure timestamp.
    pub timestamp: Timestamp,
    /// Typed failure code.
    pub code: FeatureFailureCode,
    /// Concise operator-safe message.
    pub message: String,
}

/// Feature service became idle or completed after finite input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureServiceIdle {
    /// Idle / completed timestamp.
    pub timestamp: Timestamp,
    /// Whether finite input completed.
    pub completed: bool,
}

/// Feature service stopped cleanly or after cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureServiceStopped {
    /// Stop timestamp.
    pub timestamp: Timestamp,
}

/// Perception service started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerceptionServiceStarted {
    /// Start timestamp.
    pub timestamp: Timestamp,
    /// Active observation profile identity.
    pub profile_id: String,
    /// Active observation profile version.
    pub profile_version: u32,
}

/// A channel-change observation was created (metadata only).
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelChangeObserved {
    /// Observation identity.
    pub observation_id: u64,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Source feature-vector identity.
    pub feature_vector_id: u64,
    /// Inclusive first sequence.
    pub first_sequence: u64,
    /// Inclusive last sequence.
    pub last_sequence: u64,
    /// Observation state label (`stable`, `changing`, `highly_changing`, `indeterminate`).
    pub state: String,
    /// Heuristic channel-change activity score (not a probability).
    pub activity_score: f64,
    /// Distance from the nearest classification threshold.
    pub threshold_margin: f64,
    /// Observation profile identity.
    pub profile_id: String,
    /// Observation profile version.
    pub profile_version: u32,
    /// Warning count retained on the observation.
    pub warning_count: u32,
    /// Creation timestamp.
    pub created_at: Timestamp,
}

/// Machine-readable perception / observation failure codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservationFailureCode {
    /// Feature schema incompatible with the observation profile.
    IncompatibleFeatureSchema,
    /// Required features unavailable.
    MissingFeatures,
    /// Invalid observation profile.
    InvalidProfile,
    /// Non-finite score or inputs.
    NonFinite,
    /// Output validation failure.
    OutputValidation,
    /// Service-level failure.
    ServiceFailure,
}

impl ObservationFailureCode {
    /// Stable wire label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IncompatibleFeatureSchema => "incompatible_feature_schema",
            Self::MissingFeatures => "missing_features",
            Self::InvalidProfile => "invalid_profile",
            Self::NonFinite => "non_finite",
            Self::OutputValidation => "output_validation",
            Self::ServiceFailure => "service_failure",
        }
    }
}

/// Observation creation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationFailed {
    /// Feature-vector identity when available.
    pub feature_vector_id: Option<u64>,
    /// Source sensor identifier when available.
    pub sensor_id: Option<SensorId>,
    /// Inclusive first sequence when available.
    pub first_sequence: Option<u64>,
    /// Inclusive last sequence when available.
    pub last_sequence: Option<u64>,
    /// Failure timestamp.
    pub timestamp: Timestamp,
    /// Typed failure code.
    pub code: ObservationFailureCode,
    /// Concise operator-safe message.
    pub message: String,
}

/// Perception service became idle or completed after finite input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerceptionServiceIdle {
    /// Idle / completed timestamp.
    pub timestamp: Timestamp,
    /// Whether finite input completed.
    pub completed: bool,
}

/// Perception service stopped cleanly or after cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerceptionServiceStopped {
    /// Stop timestamp.
    pub timestamp: Timestamp,
}

/// A frame was received from a sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameReceived {
    /// Received frame identifier.
    pub frame_id: FrameId,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Acquisition timestamp.
    pub timestamp: Timestamp,
    /// Monotonic sequence number within the sensor stream.
    pub sequence: u64,
}

/// A sensor plugin started producing frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensorStarted {
    /// Sensor that started.
    pub sensor_id: SensorId,
    /// Start timestamp.
    pub timestamp: Timestamp,
}

/// A sensor plugin stopped producing frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensorStopped {
    /// Sensor that stopped.
    pub sensor_id: SensorId,
    /// Stop timestamp.
    pub timestamp: Timestamp,
}

/// Classification of a sensor failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorFailureKind {
    /// The producer task exited unexpectedly.
    ProducerExited,
    /// Publishing a frame event failed.
    PublishFailed,
}

/// A sensor plugin entered a failure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensorFailed {
    /// Sensor that failed.
    pub sensor_id: SensorId,
    /// Failure timestamp.
    pub timestamp: Timestamp,
    /// Failure classification.
    pub kind: SensorFailureKind,
}

/// A new observation was recorded.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationRecorded {
    /// Recorded observation.
    pub observation: Observation,
}

/// An entity was added or updated in the world model.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityUpserted {
    /// Updated world entity.
    pub entity: WorldEntity,
}

/// An entity was removed from the world model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityRemoved {
    /// Removed entity identifier.
    pub entity_id: EntityId,
    /// Removal timestamp.
    pub timestamp: Timestamp,
}

/// A relationship was added or updated in the world model.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipUpserted {
    /// Updated relationship.
    pub relationship: WorldRelationship,
}

/// A pipeline stage completed processing for a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageCompleted {
    /// Completed stage identifier.
    pub stage_id: PipelineStageId,
    /// Processed frame identifier.
    pub frame_id: FrameId,
    /// Completion timestamp.
    pub timestamp: Timestamp,
}

/// A new world snapshot was committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSnapshotCommitted {
    /// Snapshot timestamp.
    pub timestamp: Timestamp,
    /// Optional mission context.
    pub mission_id: Option<MissionId>,
    /// Number of entities in the committed snapshot.
    pub entity_count: usize,
    /// Number of observations in the committed snapshot.
    pub observation_count: usize,
}

/// Domain events exchanged between subsystems.
///
/// Variants are explicit structs and enums so subscribers never rely on
/// string parsing or dynamically typed payloads.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A sensor frame was received.
    FrameReceived(FrameReceived),
    /// A sensor started producing frames.
    SensorStarted(SensorStarted),
    /// A sensor stopped producing frames.
    SensorStopped(SensorStopped),
    /// A sensor entered a failure state.
    SensorFailed(SensorFailed),
    /// CSI replay started producing frames.
    CsiReplayStarted(CsiReplayStarted),
    /// A CSI frame metadata event was received.
    CsiFrameReceived(CsiFrameReceived),
    /// CSI replay completed a finite fixture pass.
    CsiReplayCompleted(CsiReplayCompleted),
    /// CSI replay stopped.
    CsiReplayStopped(CsiReplayStopped),
    /// CSI replay failed.
    CsiReplayFailed(CsiReplayFailed),
    /// Calibration service started.
    CalibrationStarted(CalibrationStarted),
    /// A CSI frame was calibrated successfully (metadata only).
    CsiFrameCalibrated(CsiFrameCalibrated),
    /// Calibration failed for a frame or service fault.
    CalibrationFailed(CalibrationFailed),
    /// Calibration service stopped.
    CalibrationServiceStopped(CalibrationServiceStopped),
    /// DSP service started.
    DspServiceStarted(DspServiceStarted),
    /// A temporal CSI window was assembled (metadata only).
    CsiWindowAssembled(CsiWindowAssembled),
    /// A DSP window was processed (metadata only).
    DspWindowProcessed(DspWindowProcessed),
    /// DSP processing failed.
    DspProcessingFailed(DspProcessingFailed),
    /// DSP service became idle or completed.
    DspServiceIdle(DspServiceIdle),
    /// DSP service stopped.
    DspServiceStopped(DspServiceStopped),
    /// Feature extraction service started.
    FeatureServiceStarted(FeatureServiceStarted),
    /// A feature vector was produced (metadata only).
    FeatureVectorProduced(FeatureVectorProduced),
    /// Feature extraction failed.
    FeatureExtractionFailed(FeatureExtractionFailed),
    /// Feature service became idle or completed.
    FeatureServiceIdle(FeatureServiceIdle),
    /// Feature service stopped.
    FeatureServiceStopped(FeatureServiceStopped),
    /// Perception service started.
    PerceptionServiceStarted(PerceptionServiceStarted),
    /// A channel-change observation was created (metadata only).
    ChannelChangeObserved(ChannelChangeObserved),
    /// Observation creation failed.
    ObservationFailed(ObservationFailed),
    /// Perception service became idle or completed.
    PerceptionServiceIdle(PerceptionServiceIdle),
    /// Perception service stopped.
    PerceptionServiceStopped(PerceptionServiceStopped),
    /// An observation was recorded.
    ObservationRecorded(ObservationRecorded),
    /// An entity was added or updated.
    EntityUpserted(EntityUpserted),
    /// An entity was removed.
    EntityRemoved(EntityRemoved),
    /// A relationship was added or updated.
    RelationshipUpserted(RelationshipUpserted),
    /// A pipeline stage completed.
    StageCompleted(StageCompleted),
    /// A world snapshot was committed.
    WorldSnapshotCommitted(WorldSnapshotCommitted),
}

impl Event {
    /// Returns the primary timestamp associated with the event.
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Self::FrameReceived(event) => event.timestamp,
            Self::SensorStarted(event) => event.timestamp,
            Self::SensorStopped(event) => event.timestamp,
            Self::SensorFailed(event) => event.timestamp,
            Self::CsiReplayStarted(event) => event.timestamp,
            Self::CsiFrameReceived(event) => event.receive_timestamp,
            Self::CsiReplayCompleted(event) => event.timestamp,
            Self::CsiReplayStopped(event) => event.timestamp,
            Self::CsiReplayFailed(event) => event.timestamp,
            Self::CalibrationStarted(event) => event.timestamp,
            Self::CsiFrameCalibrated(event) => event.calibrated_at,
            Self::CalibrationFailed(event) => event.timestamp,
            Self::CalibrationServiceStopped(event) => event.timestamp,
            Self::DspServiceStarted(event) => event.timestamp,
            Self::CsiWindowAssembled(event) => event.timestamp,
            Self::DspWindowProcessed(event) => event.processed_at,
            Self::DspProcessingFailed(event) => event.timestamp,
            Self::DspServiceIdle(event) => event.timestamp,
            Self::DspServiceStopped(event) => event.timestamp,
            Self::FeatureServiceStarted(event) => event.timestamp,
            Self::FeatureVectorProduced(event) => event.extracted_at,
            Self::FeatureExtractionFailed(event) => event.timestamp,
            Self::FeatureServiceIdle(event) => event.timestamp,
            Self::FeatureServiceStopped(event) => event.timestamp,
            Self::PerceptionServiceStarted(event) => event.timestamp,
            Self::ChannelChangeObserved(event) => event.created_at,
            Self::ObservationFailed(event) => event.timestamp,
            Self::PerceptionServiceIdle(event) => event.timestamp,
            Self::PerceptionServiceStopped(event) => event.timestamp,
            Self::ObservationRecorded(event) => event.observation.timestamp,
            Self::EntityUpserted(event) => event.entity.last_updated,
            Self::EntityRemoved(event) => event.timestamp,
            Self::RelationshipUpserted(event) => event.relationship.last_updated,
            Self::StageCompleted(event) => event.timestamp,
            Self::WorldSnapshotCommitted(event) => event.timestamp,
        }
    }
}

/// Publishes domain events to the platform event bus.
///
/// Acquisition, perception, and storage subsystems publish through this
/// interface so transport details remain outside the domain layer.
pub trait EventPublisher {
    /// Error type returned when publication fails.
    type Error;

    /// Publishes a single domain event.
    fn publish(&mut self, event: Event) -> Result<(), Self::Error>;
}

/// Consumes domain events from the platform event bus.
///
/// Applications and downstream subsystems implement this trait to react to
/// state changes without polling the world model directly.
pub trait EventSubscriber {
    /// Error type returned when event handling fails.
    type Error;

    /// Handles a single domain event.
    fn on_event(&mut self, event: &Event) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityKind};
    use crate::frame::Metadata;
    use crate::ids::{EntityId, FrameId, ObservationId, SensorId};
    use crate::observation::{Confidence, Observation, ObservationValue};
    use crate::world::{RelationshipKind, WorldEntity, WorldRelationship};

    #[derive(Default)]
    struct VecPublisher {
        events: Vec<Event>,
    }

    impl EventPublisher for VecPublisher {
        type Error = ();

        fn publish(&mut self, event: Event) -> Result<(), Self::Error> {
            self.events.push(event);
            Ok(())
        }
    }

    struct CountingSubscriber {
        count: usize,
    }

    impl EventSubscriber for CountingSubscriber {
        type Error = ();

        fn on_event(&mut self, event: &Event) -> Result<(), Self::Error> {
            let _ = event.timestamp();
            self.count += 1;
            Ok(())
        }
    }

    #[test]
    fn event_timestamp_variants_are_defined() {
        let event = Event::FrameReceived(FrameReceived {
            frame_id: FrameId::new(1),
            sensor_id: SensorId::new(2),
            timestamp: Timestamp::from_nanos(99),
            sequence: 0,
        });
        assert_eq!(event.timestamp(), Timestamp::from_nanos(99));
    }

    #[test]
    fn sensor_lifecycle_events_carry_timestamps() {
        let started = Event::SensorStarted(SensorStarted {
            sensor_id: SensorId::new(1),
            timestamp: Timestamp::from_nanos(10),
        });
        let stopped = Event::SensorStopped(SensorStopped {
            sensor_id: SensorId::new(1),
            timestamp: Timestamp::from_nanos(20),
        });
        let failed = Event::SensorFailed(SensorFailed {
            sensor_id: SensorId::new(1),
            timestamp: Timestamp::from_nanos(30),
            kind: SensorFailureKind::ProducerExited,
        });
        assert_eq!(started.timestamp(), Timestamp::from_nanos(10));
        assert_eq!(stopped.timestamp(), Timestamp::from_nanos(20));
        assert_eq!(failed.timestamp(), Timestamp::from_nanos(30));
    }

    #[test]
    fn publisher_and_subscriber_traits_are_object_safe_enough_for_tests() {
        let mut publisher = VecPublisher::default();
        publisher
            .publish(Event::EntityRemoved(EntityRemoved {
                entity_id: EntityId::new(1),
                timestamp: Timestamp::from_nanos(1),
            }))
            .expect("publish succeeds");
        assert_eq!(publisher.events.len(), 1);

        let mut subscriber = CountingSubscriber { count: 0 };
        subscriber
            .on_event(&publisher.events[0])
            .expect("handle succeeds");
        assert_eq!(subscriber.count, 1);
    }

    #[test]
    fn observation_recorded_event_wraps_observation() {
        let observation = Observation {
            id: ObservationId::new(1),
            timestamp: Timestamp::from_nanos(5),
            frame_id: FrameId::new(2),
            sensor_id: SensorId::new(3),
            entity_ids: Vec::new(),
            confidence: Confidence::new(1.0).expect("valid confidence"),
            value: ObservationValue::Bool(false),
            metadata: Metadata::new(),
        };
        let event = Event::ObservationRecorded(ObservationRecorded { observation });
        assert!(matches!(event, Event::ObservationRecorded(_)));
    }

    #[test]
    fn entity_upserted_event_carries_world_entity() {
        let entity = WorldEntity {
            entity: Entity {
                id: EntityId::new(1),
                kind: EntityKind::Object,
                metadata: Metadata::new(),
            },
            confidence: Confidence::new(0.6).expect("valid confidence"),
            last_updated: Timestamp::from_nanos(1),
        };
        let event = Event::EntityUpserted(EntityUpserted { entity });
        assert_eq!(event.timestamp(), Timestamp::from_nanos(1));
    }

    #[test]
    fn relationship_event_does_not_use_string_dispatch() {
        let relationship = WorldRelationship {
            source: EntityId::new(1),
            target: EntityId::new(2),
            kind: RelationshipKind::Adjacent,
            confidence: Confidence::new(0.7).expect("valid confidence"),
            last_updated: Timestamp::from_nanos(3),
            metadata: Metadata::new(),
        };
        let event = Event::RelationshipUpserted(RelationshipUpserted { relationship });
        assert!(matches!(event, Event::RelationshipUpserted(_)));
    }

    #[test]
    fn csi_replay_events_carry_timestamps() {
        let started = Event::CsiReplayStarted(CsiReplayStarted {
            sensor_id: SensorId::new(2),
            timestamp: Timestamp::from_nanos(11),
        });
        let frame = Event::CsiFrameReceived(CsiFrameReceived {
            frame_id: FrameId::new(1),
            sensor_id: SensorId::new(2),
            sequence: 0,
            capture_timestamp: Timestamp::from_nanos(10),
            receive_timestamp: Timestamp::from_nanos(12),
            receive_antennas: 2,
            transmit_antennas: 1,
            subcarrier_count: 16,
            center_frequency_hz: Some(5_180_000_000.0),
            bandwidth_hz: Some(20_000_000.0),
            source: CsiDataSource::Replay,
            frame_token: None,
        });
        assert_eq!(started.timestamp(), Timestamp::from_nanos(11));
        assert_eq!(frame.timestamp(), Timestamp::from_nanos(12));
    }
}
