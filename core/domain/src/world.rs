//! World model data structures.

use crate::entity::Entity;
use crate::frame::Metadata;
use crate::ids::{EntityId, MissionId};
use crate::observation::{Confidence, Observation};
use crate::time::Timestamp;

/// Semantic classification of a relationship between entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RelationshipKind {
    /// Source entity spatially or logically contains the target.
    Contains,
    /// Entities are directly adjacent.
    Adjacent,
    /// Entities are associated without a stronger constraint.
    Associated,
    /// Application-defined relationship label.
    Custom(String),
}

/// A directed relationship between two entities in the world model.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldRelationship {
    /// Source entity.
    pub source: EntityId,
    /// Target entity.
    pub target: EntityId,
    /// Relationship semantics.
    pub kind: RelationshipKind,
    /// Confidence in the relationship.
    pub confidence: Confidence,
    /// Time the relationship was last updated.
    pub last_updated: Timestamp,
    /// Relationship metadata.
    pub metadata: Metadata,
}

/// An entity as represented in a world snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldEntity {
    /// Underlying entity definition.
    pub entity: Entity,
    /// Confidence in the entity's presence or state.
    pub confidence: Confidence,
    /// Time the entity was last updated in the world model.
    pub last_updated: Timestamp,
}

/// Header fields shared by every world snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldHeader {
    /// Snapshot timestamp.
    pub timestamp: Timestamp,
    /// Optional mission context.
    pub mission_id: Option<MissionId>,
    /// Snapshot-level metadata.
    pub metadata: Metadata,
}

/// A point-in-time view of the environment.
///
/// This is a data container only. Tracking, localization, and fusion logic
/// belong in higher-level subsystems.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldState {
    /// Snapshot header.
    pub header: WorldHeader,
    /// Entities present in the snapshot.
    pub entities: Vec<WorldEntity>,
    /// Relationships between entities.
    pub relationships: Vec<WorldRelationship>,
    /// Observations contributing to the snapshot.
    pub observations: Vec<Observation>,
}

impl WorldState {
    /// Creates an empty world snapshot at `timestamp`.
    pub fn empty(timestamp: Timestamp) -> Self {
        Self {
            header: WorldHeader {
                timestamp,
                mission_id: None,
                metadata: Metadata::new(),
            },
            entities: Vec::new(),
            relationships: Vec::new(),
            observations: Vec::new(),
        }
    }

    /// Returns the number of entities in the snapshot.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Returns the number of observations in the snapshot.
    pub fn observation_count(&self) -> usize {
        self.observations.len()
    }
}

/// Read-only access to the current world model.
///
/// Storage and perception subsystems implement this trait to expose snapshots
/// without prescribing how state is maintained or updated.
pub trait WorldModel {
    /// Returns the latest committed world snapshot.
    fn snapshot(&self) -> &WorldState;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityKind};
    use crate::ids::EntityId;
    use crate::observation::{Observation, ObservationValue};
    use crate::ids::{FrameId, ObservationId, SensorId};

    struct StaticWorld {
        state: WorldState,
    }

    impl WorldModel for StaticWorld {
        fn snapshot(&self) -> &WorldState {
            &self.state
        }
    }

    #[test]
    fn empty_world_state_has_zero_counts() {
        let state = WorldState::empty(Timestamp::from_nanos(1));
        assert_eq!(state.entity_count(), 0);
        assert_eq!(state.observation_count(), 0);
    }

    #[test]
    fn world_entity_wraps_entity_with_confidence() {
        let world_entity = WorldEntity {
            entity: Entity {
                id: EntityId::new(1),
                kind: EntityKind::Region,
                metadata: Metadata::new(),
            },
            confidence: Confidence::new(0.5).expect("valid confidence"),
            last_updated: Timestamp::from_nanos(2),
        };
        assert_eq!(world_entity.entity.id, EntityId::new(1));
    }

    #[test]
    fn world_model_exposes_snapshot() {
        let mut state = WorldState::empty(Timestamp::from_nanos(10));
        state.observations.push(Observation {
            id: ObservationId::new(1),
            timestamp: Timestamp::from_nanos(10),
            frame_id: FrameId::new(2),
            sensor_id: SensorId::new(3),
            entity_ids: Vec::new(),
            confidence: Confidence::new(0.8).expect("valid confidence"),
            value: ObservationValue::Bool(true),
            metadata: Metadata::new(),
        });
        let model = StaticWorld { state };
        assert_eq!(model.snapshot().observation_count(), 1);
    }
}
