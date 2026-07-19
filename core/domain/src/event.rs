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
