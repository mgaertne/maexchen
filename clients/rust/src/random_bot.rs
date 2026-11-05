//! Rolls or looks at the dice at random.

use crate::bot::{Bot, Server};

const CHANCE_TO_ROLL: f64 = 0.8;

/// Rolls or looks at the dice at random on its turn and announces what it rolled.
pub struct RandomBot;

impl Bot for RandomBot {
    fn on_message(&mut self, server: &Server, command: &str, parameters: &[&str]) {
        match (command, parameters) {
            ("YOUR TURN", [.., token]) => {
                let answer = if rand::random_bool(CHANCE_TO_ROLL) {
                    "ROLL"
                } else {
                    "SEE"
                };
                server.send(answer, &[token]);
            }
            ("ROLLED", [dice, .., token]) => server.send("ANNOUNCE", &[dice, token]),
            _ => {}
        }
    }
}
