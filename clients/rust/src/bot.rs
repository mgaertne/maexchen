//! The interface of a bot and the way it talks to the server.

use crate::transport::Transport;

/// Shows a message of the protocol the way the Python client logs it: the command followed by
/// the parameters in quotes and brackets, as in `ROUND STARTING ['token']`.
pub(crate) fn readable(message: &str) -> String {
    let mut parts = message.split(';');
    let command = parts.next().unwrap_or_default();
    let parameters: Vec<String> = parts.map(|parameter| format!("'{parameter}'")).collect();
    format!("{command} [{}]", parameters.join(", "))
}

/// How a bot takes part in the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Takes part in rounds.
    Player,
    /// Only watches.
    Spectator,
}

impl Role {
    /// The command that registers a client in this role: `REGISTER;<name>` for players and
    /// `REGISTER_SPECTATOR;<name>` for spectators.
    #[must_use]
    pub fn register_command(self) -> &'static str {
        match self {
            Self::Player => "REGISTER",
            Self::Spectator => "REGISTER_SPECTATOR",
        }
    }
}

/// The behaviour of a bot; write your own by implementing this trait.
///
/// The client registers the bot, joins rounds, answers heartbeats and unregisters it again.
/// What is left for the bot is to react to the messages of the game.
///
/// # Messages from the server
///
/// | `command` | `parameters` |
/// |---|---|
/// | `ROUND STARTING` | `[token]`, an invitation; answer with `JOIN` unless the client joins by itself |
/// | `ROUND STARTED` | `[round, players]`, the players in turn order, separated by `,` |
/// | `ROUND CANCELED` | `[reason]` |
/// | `YOUR TURN` | `[token]`; answer with `ROLL` or `SEE` |
/// | `ROLLED` | `[dice, token]`, for example `["3,1", token]`; answer with `ANNOUNCE` |
/// | `PLAYER ROLLS` | `[player]` |
/// | `PLAYER WANTS TO SEE` | `[player]` |
/// | `ANNOUNCED` | `[player, dice]` |
/// | `ACTUAL DICE` | `[dice]`, shown when somebody looked at the dice |
/// | `PLAYER LOST` | `[players, reason]` |
/// | `SCORE` | `[scores]`, as `player:points` pairs separated by `,` |
///
/// # Answers of the bot
///
/// `ROLL`, `SEE` and `JOIN` take the token of the message they answer. `ANNOUNCE` takes the
/// dice and the token of the `ROLLED` message.
///
/// # Example
///
/// A bot with state: it rolls, announces what it rolled and counts the rounds it has seen.
///
/// ```
/// use maexchen_client::bot::{Bot, Server};
///
/// struct CountingBot {
///     rounds: u32,
/// }
///
/// impl Bot for CountingBot {
///     fn on_message(&mut self, server: &Server, command: &str, parameters: &[&str]) {
///         match (command, parameters) {
///             ("ROUND STARTED", _) => self.rounds += 1,
///             ("YOUR TURN", [.., token]) => server.send("ROLL", &[token]),
///             ("ROLLED", [dice, .., token]) => server.send("ANNOUNCE", &[dice, token]),
///             _ => {}
///         }
///     }
/// }
/// #
/// # use std::{cell::RefCell, io};
/// # use maexchen_client::transport::Transport;
/// # struct Recorder(RefCell<Vec<String>>);
/// # impl Transport for Recorder {
/// #     fn send(&self, message: &str) -> io::Result<()> {
/// #         self.0.borrow_mut().push(message.to_string());
/// #         Ok(())
/// #     }
/// #     fn receive(&self) -> io::Result<Option<String>> {
/// #         Ok(None)
/// #     }
/// # }
/// # let transport = Recorder(RefCell::default());
/// # let server = Server::new(&transport);
/// # let mut bot = CountingBot { rounds: 0 };
/// bot.on_message(&server, "ROUND STARTED", &["1", "anna,bert"]);
/// bot.on_message(&server, "YOUR TURN", &["token"]);
///
/// assert_eq!(bot.rounds, 1);
/// assert_eq!(transport.0.into_inner(), ["ROLL;token"]);
/// ```
///
/// Register it as a variant of `BotType` in `main.rs` to start it with `-t`.
pub trait Bot {
    /// Whether the bot plays or only watches; spectators register differently.
    fn role(&self) -> Role {
        Role::Player
    }

    /// Called for every message of the server except heartbeats, and for invitations only
    /// when the client does not join by itself (`--no-auto-join`).
    ///
    /// `command` is the first part of the message, `parameters` the parts after it, so
    /// `YOUR TURN;4711` arrives as `"YOUR TURN"` and `["4711"]`. Answer with [`Server::send`].
    fn on_message(&mut self, server: &Server, command: &str, parameters: &[&str]);
}

/// The server a bot talks to.
pub struct Server<'a> {
    transport: &'a dyn Transport,
}

impl<'a> Server<'a> {
    /// Talks to the server over `transport`.
    pub fn new(transport: &'a dyn Transport) -> Self {
        Self { transport }
    }

    /// Sends `command` with its parameters, for example `send("ROLL", &[token])`.
    ///
    /// The parameters are joined with `;`, so `send("ANNOUNCE", &["6,5", token])` sends
    /// `ANNOUNCE;6,5;<token>`. A failing send is logged and otherwise ignored, as UDP gives no
    /// guarantee anyway.
    pub fn send<S: AsRef<str>>(&self, command: &str, parameters: &[S]) {
        let mut message = command.to_string();
        for parameter in parameters {
            message.push(';');
            message.push_str(parameter.as_ref());
        }

        log::info!(target: "send", "----> {}", readable(&message));
        if let Err(error) = self.transport.send(&message) {
            log::warn!(target: "send", "Failed to send {}: {error}", readable(&message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Role, readable};

    #[test]
    fn messages_are_logged_with_quoted_parameters_in_brackets() {
        assert_eq!(
            readable("ROUND STARTING;af99f6db"),
            "ROUND STARTING ['af99f6db']"
        );
        assert_eq!(
            readable("ROUND STARTED;514105;anna,bert"),
            "ROUND STARTED ['514105', 'anna,bert']"
        );
        assert_eq!(readable("UNREGISTER"), "UNREGISTER []");
    }

    #[test]
    fn every_role_registers_with_its_own_command() {
        assert_eq!(Role::Player.register_command(), "REGISTER");
        assert_eq!(Role::Spectator.register_command(), "REGISTER_SPECTATOR");
    }
}
