//! What the connection thread tells the window.

use core::time::Duration;

use crate::message::Message;

/// Where the connection to the server stands.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    /// Registered, or about to register.
    Connected,
    /// The connection broke; the next attempt follows after `retry_in`.
    Reconnecting {
        /// What went wrong.
        reason: String,
        /// How long the spectator waits before it tries again.
        retry_in: Duration,
    },
}

/// A change the window has to show.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Update {
    /// The server sent a message the spectator understands.
    Message(Message),
    /// The state of the connection changed.
    Status(Status),
}
