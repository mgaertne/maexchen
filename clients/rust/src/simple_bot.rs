//! Rolls on its turn and announces exactly what it rolled.

use crate::bot::{Bot, Server};

/// Rolls on its turn and announces exactly what it rolled.
pub struct SimpleBot;

impl Bot for SimpleBot {
    fn on_message(&mut self, server: &Server, command: &str, parameters: &[&str]) {
        match (command, parameters) {
            ("YOUR TURN", [.., token]) => server.send("ROLL", &[token]),
            ("ROLLED", [dice, .., token]) => server.send("ANNOUNCE", &[dice, token]),
            _ => {}
        }
    }
}
