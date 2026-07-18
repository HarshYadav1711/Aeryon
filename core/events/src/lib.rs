//! Typed in-process event bus for the Aeryon perception platform.
//!
//! The bus delivers strongly typed [`aeryon_domain::Event`] values to multiple
//! subscribers using a Tokio broadcast channel. It is intentionally
//! non-durable and in-process only.

#![deny(missing_docs)]

use core::fmt;

use aeryon_domain::{Event, EventPublisher};
use tokio::sync::broadcast;

/// Default capacity for the broadcast channel.
pub const DEFAULT_CAPACITY: usize = 256;

/// Errors produced when publishing or receiving events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusError {
    /// No active subscribers; the event was dropped.
    NoSubscribers,
    /// The bus has been closed.
    Closed,
    /// The receiver lagged and missed one or more events.
    Lagged(u64),
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSubscribers => f.write_str("event bus has no subscribers"),
            Self::Closed => f.write_str("event bus is closed"),
            Self::Lagged(n) => write!(f, "event bus receiver lagged by {n} events"),
        }
    }
}

impl std::error::Error for BusError {}

/// Publisher handle for the typed event bus.
#[derive(Clone, Debug)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    /// Creates an event bus with `DEFAULT_CAPACITY`.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Creates an event bus with the given broadcast capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    /// Publishes a domain event to all subscribers.
    pub fn publish(&self, event: Event) -> Result<usize, BusError> {
        match self.sender.send(event) {
            Ok(receivers) => Ok(receivers),
            Err(_) => Err(BusError::NoSubscribers),
        }
    }

    /// Subscribes to domain events.
    pub fn subscribe(&self) -> EventReceiver {
        EventReceiver {
            receiver: self.sender.subscribe(),
        }
    }

    /// Returns the number of active receivers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventPublisher for EventBus {
    type Error = BusError;

    fn publish(&mut self, event: Event) -> Result<(), Self::Error> {
        EventBus::publish(self, event).map(|_| ())
    }
}

/// Subscriber handle for the typed event bus.
#[derive(Debug)]
pub struct EventReceiver {
    receiver: broadcast::Receiver<Event>,
}

impl EventReceiver {
    /// Receives the next event.
    pub async fn recv(&mut self) -> Result<Event, BusError> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Closed) => Err(BusError::Closed),
            Err(broadcast::error::RecvError::Lagged(n)) => Err(BusError::Lagged(n)),
        }
    }

    /// Attempts to receive a pending event without waiting.
    ///
    /// Returns `None` when the channel is empty.
    pub fn try_recv(&mut self) -> Result<Option<Event>, BusError> {
        match self.receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(broadcast::error::TryRecvError::Empty) => Ok(None),
            Err(broadcast::error::TryRecvError::Closed) => Err(BusError::Closed),
            Err(broadcast::error::TryRecvError::Lagged(n)) => Err(BusError::Lagged(n)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeryon_domain::{FrameId, FrameReceived, SensorId, Timestamp};

    fn sample_event(sequence: u64) -> Event {
        Event::FrameReceived(FrameReceived {
            frame_id: FrameId::new(sequence),
            sensor_id: SensorId::new(1),
            timestamp: Timestamp::from_nanos(sequence),
            sequence,
        })
    }

    #[tokio::test]
    async fn published_event_reaches_subscriber() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();
        bus.publish(sample_event(1)).expect("publish");
        let event = receiver.recv().await.expect("recv");
        assert!(matches!(
            event,
            Event::FrameReceived(FrameReceived { sequence: 1, .. })
        ));
    }

    #[tokio::test]
    async fn multiple_subscribers_receive_broadcast() {
        let bus = EventBus::new();
        let mut first = bus.subscribe();
        let mut second = bus.subscribe();
        bus.publish(sample_event(7)).expect("publish");
        assert!(matches!(
            first.recv().await.expect("first"),
            Event::FrameReceived(FrameReceived { sequence: 7, .. })
        ));
        assert!(matches!(
            second.recv().await.expect("second"),
            Event::FrameReceived(FrameReceived { sequence: 7, .. })
        ));
    }

    #[test]
    fn publish_without_subscribers_returns_typed_error() {
        let bus = EventBus::new();
        let error = bus.publish(sample_event(1)).expect_err("no subscribers");
        assert_eq!(error, BusError::NoSubscribers);
    }

    #[tokio::test]
    async fn typed_events_remain_intact() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();
        let published = sample_event(42);
        bus.publish(published.clone()).expect("publish");
        let received = receiver.recv().await.expect("recv");
        assert_eq!(received, published);
    }
}
