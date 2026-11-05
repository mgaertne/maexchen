//! A client for the Maexchen server.
//!
//! To write your own bot, implement [`bot::Bot`], add it next to [`simple_bot::SimpleBot`] and
//! register it in `BotType` in `main.rs`. The trait documents the messages the server sends; a
//! bot answers with [`bot::Server::send`]. The client itself ([`client`]) registers the bot,
//! joins rounds and answers heartbeats, and [`transport`] carries the messages to and from the
//! server.

#![warn(missing_docs)]

pub mod bot;
pub mod client;
pub mod random_bot;
pub mod simple_bot;
pub mod spectator_bot;
pub mod transport;
