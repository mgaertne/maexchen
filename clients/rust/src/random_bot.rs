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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::RandomBot;
    use crate::{
        bot::{Bot, Server},
        transport::testing::FakeTransport,
    };

    fn answers(command: &str, parameters: &[&str]) -> Vec<String> {
        let transport = FakeTransport::new(&[]);
        RandomBot.on_message(&Server::new(&transport), command, parameters);
        transport.sent()
    }

    #[test]
    fn it_rolls_or_looks_on_its_turn() {
        let seen: HashSet<String> = (0..200)
            .flat_map(|_| answers("YOUR TURN", &["token-1"]))
            .collect();

        assert_eq!(
            seen,
            HashSet::from(["ROLL;token-1".to_string(), "SEE;token-1".to_string()])
        );
    }

    #[test]
    fn it_announces_what_it_rolled() {
        assert_eq!(
            answers("ROLLED", &["3,1", "token-2"]),
            ["ANNOUNCE;3,1;token-2"]
        );
    }

    #[test]
    fn it_ignores_everything_else() {
        assert!(answers("SCORE", &["anna:1"]).is_empty());
        assert!(answers("YOUR TURN", &[]).is_empty());
    }
}
