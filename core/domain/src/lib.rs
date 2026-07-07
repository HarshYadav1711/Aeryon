//! Core domain contracts for the Aeryon perception platform.
//!
//! This crate defines the shared language—identifiers, timestamps, frames,
//! observations, world state, events, and pipeline stages—that every subsystem
//! depends upon. It contains data models and traits only; acquisition,
//! processing, and persistence logic belong in downstream crates.

#![deny(missing_docs)]

pub mod entity;
pub mod event;
pub mod frame;
pub mod ids;
pub mod observation;
pub mod pipeline;
pub mod sensor;
pub mod time;
pub mod world;

pub use entity::{Entity, EntityKind};
pub use event::{
    EntityRemoved, EntityUpserted, Event, EventPublisher, EventSubscriber, FrameReceived,
    ObservationRecorded, RelationshipUpserted, StageCompleted, WorldSnapshotCommitted,
};
pub use frame::{Frame, FrameMetadata, Metadata, MetadataKey, MetadataValue};
pub use ids::{EntityId, FrameId, MissionId, ObservationId, SensorId};
pub use observation::{Confidence, Observation, ObservationValue};
pub use pipeline::{PipelineStage, PipelineStageId, PipelineStageKind};
pub use sensor::Sensor;
pub use time::Timestamp;
pub use world::{
    RelationshipKind, WorldEntity, WorldHeader, WorldModel, WorldRelationship, WorldState,
};
