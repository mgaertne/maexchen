//! Maexchen game server: a pure game state machine driven by a UDP front end.
//!
//! `Game` holds the rules and does no I/O. It consumes `Event`s and answers with `Effect`s
//! (messages to send, timers to start), which makes it easy to test with a scripted `Dealer`.
//! `serve` connects a game to a UDP socket.
//!
//! The rules are chosen with a `RuleSet`: fixed rules for every round, or random rules that are
//! decided anew for every round (Mia is 2,1 or 4,2, and blind rolls may be allowed). Random rules
//! are announced as `ROUND STARTED;<round>;<rules>;<players>`, where `<rules>` is `mia21`,
//! `mia42` or `mia42,blind`. With fixed rules the message stays
//! `ROUND STARTED;<round>;<players>`.

#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]

mod dealer;
mod dice;
mod game;
mod interface;
mod registry;
mod rules;
mod score;
mod server;

pub use dealer::{Dealer, RandomDealer};
pub use dice::{ParseDiceError, SimpleDice};
pub use game::{Effect, Event, Game, TimerId};
pub use interface::{
    CancellationReason, ClientCommand, LosingReason, ParseCommandError, PlayerMessage,
    RejectionReason,
};
pub use rules::{BlindRoll, MiaRule, ParseRulesError, RuleSet, Rules};
pub use server::serve;

pub(crate) mod prelude {
    pub(crate) use super::{
        CancellationReason, ClientCommand, LosingReason, PlayerMessage, RejectionReason,
    };
}
