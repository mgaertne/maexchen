//! Watches the game without taking part.

use crate::bot::{Bot, Role, Server};

/// Only watches the game.
pub struct SpectatorBot;

impl Bot for SpectatorBot {
    fn role(&self) -> Role {
        Role::Spectator
    }

    fn on_message(&mut self, _server: &Server, _command: &str, _parameters: &[&str]) {}
}
