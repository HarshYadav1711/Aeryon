//! Entity definitions used by the world model.

use std::borrow::Cow;

use crate::frame::Metadata;
use crate::ids::EntityId;

/// Semantic classification of an entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityKind {
    /// A discrete object in the environment.
    Object,
    /// A spatial region or zone.
    Region,
    /// An autonomous or semi-autonomous agent.
    Agent,
    /// Application-defined entity classification.
    Custom(Cow<'static, str>),
}

/// A domain entity independent of any particular world snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Stable entity identifier.
    pub id: EntityId,
    /// Semantic classification.
    pub kind: EntityKind,
    /// Entity-level metadata.
    pub metadata: Metadata,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntityId;

    #[test]
    fn entity_carries_kind_and_metadata() {
        let entity = Entity {
            id: EntityId::new(10),
            kind: EntityKind::Object,
            metadata: Metadata::new(),
        };
        assert_eq!(entity.id, EntityId::new(10));
        assert_eq!(entity.kind, EntityKind::Object);
        assert!(entity.metadata.is_empty());
    }

    #[test]
    fn custom_entity_kind_round_trips_label() {
        let kind = EntityKind::Custom(Cow::Borrowed("landmark"));
        assert_eq!(kind, EntityKind::Custom(Cow::Borrowed("landmark")));
    }
}
