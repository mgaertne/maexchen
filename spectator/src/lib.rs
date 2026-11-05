//! A spectator for the Maexchen server that shows scores and rounds live.
//!
//! The connection runs in a thread of its own ([`connection`]) on top of the client library and
//! hands the messages it understands ([`message`]) to the window ([`app`]).

#![warn(missing_docs)]
#![warn(clippy::pedantic)]

pub mod app;
pub mod board;
pub mod bot;
pub mod connection;
pub mod message;
pub mod model;
pub mod settings;
pub mod texts;
pub mod update;
