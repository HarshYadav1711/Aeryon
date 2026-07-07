//! Strongly typed domain events.

use crate::ids::{EntityId, FrameId, MissionId, SensorId};
use crate::observation::Observation;
use crate::pipeline::PipelineStageId;
use crate::time::Timestamp;
use crate::world::{WorldEntity, WorldRelationship};

/// A frame was received from a sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameReceived {
    /// Received frame identifier.
    pub frame_id: FrameId,
    /// Source sensor identifier.
    pub sensor_id: SensorId,
    /// Acquisition timestamp.
    pub timestamp: Timestamp,
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
        });
        assert_eq!(event.timestamp(), Timestamp::from_nanos(99));
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
}
