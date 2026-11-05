//! The generic part of every bot: register, pass the messages on, unregister.

use core::sync::atomic::{AtomicBool, Ordering};
use std::{
    io,
    time::{Duration, Instant},
};

use thiserror::Error;

use crate::{
    bot::{Bot, Role, Server, readable},
    transport::Transport,
};

/// How long the server may stay silent before the client gives up.
pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(5);
/// How long the client waits for the server to answer a registration.
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(2);

/// Why a client stopped with an error.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ClientError {
    /// The server refused the registration; carries the message of the server.
    #[error("the server rejected the registration: {0}")]
    RegistrationRejected(String),
    /// The server did not answer the registration in time.
    #[error("the server did not answer the registration")]
    RegistrationUnanswered,
    /// The server stayed silent for longer than the allowed time.
    #[error("the server sent no heartbeat for {0:?}")]
    HeartbeatMissing(Duration),
    /// Sending to or receiving from the server failed.
    #[error("lost the connection to the server")]
    ConnectionLost(#[from] io::Error),
}

/// How the client takes part in the game, independent of the bot.
pub struct Settings {
    /// The name to register with.
    pub name: String,
    /// Whether the client answers invitations by itself instead of passing them to the bot.
    pub auto_join: bool,
    /// How long the server may stay silent before the client gives up; `None` never gives up.
    pub heartbeat_timeout: Option<Duration>,
    /// How long the client waits for the answer to a registration or unregistration.
    pub answer_timeout: Duration,
}

enum Answer {
    Confirmed,
    Rejected(String),
    TimedOut,
}

/// Registers the bot, hands it all messages until `stop` is set or the server falls silent,
/// and unregisters it again.
pub fn run(
    transport: &dyn Transport,
    bot: &mut dyn Bot,
    settings: &Settings,
    stop: &AtomicBool,
) -> Result<(), ClientError> {
    let server = Server::new(transport);
    register(transport, &server, bot.role(), settings)?;
    let registration = Registration {
        transport,
        server: &server,
        answer_timeout: settings.answer_timeout,
        finished: false,
    };
    let outcome = play(transport, &server, bot, settings, stop);
    registration.unregister();
    outcome
}

/// A registration that unregisters itself when it is dropped without [`Registration::unregister`],
/// for example while a panic in a bot unwinds, so that the server does not keep a dead client.
struct Registration<'a> {
    transport: &'a dyn Transport,
    server: &'a Server<'a>,
    answer_timeout: Duration,
    finished: bool,
}

impl Registration<'_> {
    fn unregister(mut self) {
        self.finished = true;
        unregister(self.transport, self.server, self.answer_timeout);
    }
}

impl Drop for Registration<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.server.send("UNREGISTER", &[] as &[&str]);
        }
    }
}

fn register(
    transport: &dyn Transport,
    server: &Server,
    role: Role,
    settings: &Settings,
) -> Result<(), ClientError> {
    server.send(role.register_command(), &[&settings.name]);

    match await_answer(transport, "REGISTERED", settings.answer_timeout)? {
        Answer::Confirmed => Ok(()),
        Answer::Rejected(message) => Err(ClientError::RegistrationRejected(message)),
        Answer::TimedOut => Err(ClientError::RegistrationUnanswered),
    }
}

fn play(
    transport: &dyn Transport,
    server: &Server,
    bot: &mut dyn Bot,
    settings: &Settings,
    stop: &AtomicBool,
) -> Result<(), ClientError> {
    let mut last_heartbeat = Instant::now();
    while !stop.load(Ordering::Acquire) {
        if let Some(limit) = settings.heartbeat_timeout
            && last_heartbeat.elapsed() > limit
        {
            return Err(ClientError::HeartbeatMissing(limit));
        }

        let Some(message) = transport.receive()? else {
            continue;
        };
        let parts: Vec<&str> = message.split(';').collect();
        let Some((&command, parameters)) = parts.split_first() else {
            continue;
        };

        if command == "HEARTBEAT" {
            log::debug!(target: "receive", "<---- {}", readable(&message));
            last_heartbeat = Instant::now();
            continue;
        }

        log::info!(target: "receive", "<---- {}", readable(&message));
        match (command, parameters) {
            ("ROUND STARTING", [.., token]) if settings.auto_join => server.send("JOIN", &[token]),
            _ => bot.on_message(server, command, parameters),
        }
    }
    Ok(())
}

fn unregister(transport: &dyn Transport, server: &Server, timeout: Duration) {
    server.send("UNREGISTER", &[] as &[&str]);
    if !matches!(
        await_answer(transport, "UNREGISTERED", timeout),
        Ok(Answer::Confirmed)
    ) {
        log::warn!(target: "client", "The server did not confirm the unregistration.");
    }
}

fn await_answer(
    transport: &dyn Transport,
    confirmation: &str,
    timeout: Duration,
) -> io::Result<Answer> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let Some(message) = transport.receive()? else {
            continue;
        };
        log::info!(target: "receive", "<---- {}", readable(&message));
        match message.split(';').next() {
            Some(command) if command == confirmation => return Ok(Answer::Confirmed),
            Some("REJECTED") => return Ok(Answer::Rejected(message)),
            _ => {}
        }
    }
    Ok(Answer::TimedOut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::testing::FakeTransport;

    #[derive(Default)]
    struct RecordingBot {
        role: Option<Role>,
        messages: Vec<String>,
    }

    impl Bot for RecordingBot {
        fn role(&self) -> Role {
            self.role.unwrap_or(Role::Player)
        }

        fn on_message(&mut self, _server: &Server, command: &str, parameters: &[&str]) {
            self.messages.push(format!("{command} {parameters:?}"));
        }
    }

    fn settings() -> Settings {
        Settings {
            name: "anna".to_string(),
            auto_join: true,
            heartbeat_timeout: None,
            answer_timeout: Duration::from_millis(50),
        }
    }

    fn play_through(
        transport: &FakeTransport,
        bot: &mut RecordingBot,
        settings: &Settings,
    ) -> Result<(), ClientError> {
        run(transport, bot, settings, &transport.stop)
    }

    #[test]
    fn registers_a_player_and_unregisters_at_the_end() {
        let transport = FakeTransport::new(&[]);

        play_through(&transport, &mut RecordingBot::default(), &settings()).unwrap();

        assert_eq!(transport.sent(), ["REGISTER;anna", "UNREGISTER"]);
    }

    #[test]
    fn registers_a_spectator_as_such() {
        let transport = FakeTransport::new(&[]);
        let mut bot = RecordingBot {
            role: Some(Role::Spectator),
            ..RecordingBot::default()
        };

        play_through(&transport, &mut bot, &settings()).unwrap();

        assert_eq!(transport.sent()[0], "REGISTER_SPECTATOR;anna");
    }

    #[test]
    fn a_rejected_registration_is_an_error_and_needs_no_unregistration() {
        let transport = FakeTransport::new(&[]).rejecting();

        let error =
            play_through(&transport, &mut RecordingBot::default(), &settings()).unwrap_err();

        assert!(
            matches!(&error, ClientError::RegistrationRejected(message) if message.contains("NAME_ALREADY_TAKEN")),
            "{error}"
        );
        assert_eq!(transport.sent(), ["REGISTER;anna"]);
    }

    struct PanickingBot;

    impl Bot for PanickingBot {
        fn on_message(&mut self, _server: &Server, _command: &str, _parameters: &[&str]) {
            panic!("a bug in the bot");
        }
    }

    #[test]
    fn unregisters_when_a_bot_panics() {
        let transport = FakeTransport::new(&["YOUR TURN;token"]);

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&transport, &mut PanickingBot, &settings(), &transport.stop)
        }));

        assert!(outcome.is_err());
        assert_eq!(transport.sent(), ["REGISTER;anna", "UNREGISTER"]);
    }

    #[test]
    fn unregisters_only_once_without_a_panic() {
        let transport = FakeTransport::new(&[]);

        play_through(&transport, &mut RecordingBot::default(), &settings()).unwrap();

        assert_eq!(transport.sent(), ["REGISTER;anna", "UNREGISTER"]);
    }

    #[test]
    fn a_registration_without_an_answer_is_an_error() {
        let transport = FakeTransport::new(&[]).unresponsive();

        let error =
            play_through(&transport, &mut RecordingBot::default(), &settings()).unwrap_err();

        assert!(
            matches!(error, ClientError::RegistrationUnanswered),
            "{error}"
        );
        assert_eq!(transport.sent(), ["REGISTER;anna"]);
    }

    #[test]
    fn a_broken_connection_is_an_error_with_its_cause() {
        let transport = FakeTransport::new(&[]).broken();

        let error =
            play_through(&transport, &mut RecordingBot::default(), &settings()).unwrap_err();

        assert!(
            matches!(&error, ClientError::ConnectionLost(cause) if cause.kind() == io::ErrorKind::ConnectionReset),
            "{error}"
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn hands_messages_without_heartbeats_to_the_bot() {
        let transport = FakeTransport::new(&["HEARTBEAT", "YOUR TURN;token", "SCORE;a:1,b:0"]);
        let mut bot = RecordingBot::default();

        play_through(&transport, &mut bot, &settings()).unwrap();

        assert_eq!(
            bot.messages,
            ["YOUR TURN [\"token\"]", "SCORE [\"a:1,b:0\"]"]
        );
    }

    #[test]
    fn joins_announced_rounds_by_itself() {
        let transport = FakeTransport::new(&["ROUND STARTING;token"]);
        let mut bot = RecordingBot::default();

        play_through(&transport, &mut bot, &settings()).unwrap();

        assert_eq!(transport.sent()[1], "JOIN;token");
        assert!(bot.messages.is_empty());
    }

    #[test]
    fn leaves_joining_to_the_bot_without_auto_join() {
        let transport = FakeTransport::new(&["ROUND STARTING;token"]);
        let mut bot = RecordingBot::default();
        let settings = Settings {
            auto_join: false,
            ..settings()
        };

        play_through(&transport, &mut bot, &settings).unwrap();

        assert_eq!(transport.sent(), ["REGISTER;anna", "UNREGISTER"]);
        assert_eq!(bot.messages, ["ROUND STARTING [\"token\"]"]);
    }

    #[test]
    fn gives_up_when_the_heartbeats_stop() {
        let transport = FakeTransport::new(&[]).never_ending();
        let settings = Settings {
            heartbeat_timeout: Some(Duration::from_millis(30)),
            ..settings()
        };

        let error = play_through(&transport, &mut RecordingBot::default(), &settings).unwrap_err();

        assert!(matches!(error, ClientError::HeartbeatMissing(_)), "{error}");
        assert_eq!(transport.sent().last().unwrap(), "UNREGISTER");
    }
}
