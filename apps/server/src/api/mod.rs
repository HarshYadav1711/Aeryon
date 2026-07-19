//! Local-development HTTP API over the running application state.
//!
//! This module is a read-only boundary. It does not own a second event bus,
//! plugin runtime, or statistics store.

mod dto;
mod error;
mod routes;
mod state;
mod time;

pub use routes::serve;
pub use state::AppState;

#[cfg(test)]
mod tests;
