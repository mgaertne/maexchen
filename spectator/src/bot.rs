//! The spectator as a bot of the client library.

use std::sync::mpsc::Sender;

use maexchen_client::bot::{Bot, Role, Server};

use crate::{message::parse, update::Update};

/// Registers as spectator and passes every message it understands on to the window.
#[derive(Debug)]
pub struct SpectatorBot {
    updates: Sender<Update>,
}

impl SpectatorBot {
    /// A bot that reports to `updates`.
    #[must_use]
    pub fn new(updates: Sender<Update>) -> Self {
        Self { updates }
    }
}

impl Bot for SpectatorBot {
    fn role(&self) -> Role {
        Role::Spectator
    }

    fn on_message(&mut self, _server: &Server, command: &str, parameters: &[&str]) {
        if let Some(message) = parse(command, parameters) {
            let _window_is_gone = self.updates.send(Update::Message(message));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use maexchen_client::{
        bot::{Bot, Role, Server},
        transport::Transport,
    };

    use super::SpectatorBot;
    use crate::{message::Message, update::Update};

    struct Silent;

    impl Transport for Silent {
        fn send(&self, _message: &str) -> std::io::Result<()> {
            Ok(())
        }

        fn receive(&self) -> std::io::Result<Option<String>> {
            Ok(None)
        }
    }

    #[test]
    fn it_registers_as_a_spectator() {
        let (sender, _receiver) = mpsc::channel();

        assert_eq!(SpectatorBot::new(sender).role(), Role::Spectator);
    }

    #[test]
    fn it_passes_known_messages_on_and_drops_the_rest() {
        let (sender, receiver) = mpsc::channel();
        let mut bot = SpectatorBot::new(sender);
        let server = Server::new(&Silent);

        bot.on_message(&server, "PLAYER ROLLS", &["anna"]);
        bot.on_message(&server, "SOMETHING NEW", &["42"]);

        assert_eq!(
            receiver.try_iter().collect::<Vec<_>>(),
            [Update::Message(Message::PlayerRolls("anna".to_string()))]
        );
    }

    #[test]
    fn it_survives_a_closed_window() {
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        let mut bot = SpectatorBot::new(sender);

        bot.on_message(&Server::new(&Silent), "PLAYER ROLLS", &["anna"]);
    }
}
