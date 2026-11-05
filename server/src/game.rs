use core::{net::SocketAddr, time::Duration};

use state::State;
use table::Table;

use crate::{
    dealer::Dealer,
    prelude::{ClientCommand, PlayerMessage},
    registry::Role,
    rules::RuleSet,
};

mod lobby;
mod round;
mod state;
mod table;
mod turns;

/// Identifies a timer so that late timer events can be told apart.
pub type TimerId = u64;

/// Everything that can happen to the game from the outside.
#[derive(Debug)]
#[non_exhaustive]
pub enum Event {
    /// Starts the first round.
    Start,
    /// Time to send heartbeats to all registered clients.
    Heartbeat,
    /// A timer started by [`Effect::StartTimer`] has run out.
    TimerFired(TimerId),
    /// A client sent a command.
    Received {
        /// Where the command came from.
        from: SocketAddr,
        /// The parsed command.
        command: ClientCommand,
    },
}

/// What the surrounding I/O layer has to do after an [`Event`].
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Effect {
    /// Send the message to the address.
    Send {
        /// The receiver.
        to: SocketAddr,
        /// What to tell it.
        message: PlayerMessage,
    },
    /// Report [`Event::TimerFired`] with the id after the duration.
    StartTimer {
        /// The id to report back.
        id: TimerId,
        /// How long to wait.
        after: Duration,
    },
}

/// The rules of Mäxchen as a pure state machine over the registered clients.
///
/// It performs no I/O and starts no timers itself: [`Game::handle`] returns the
/// [`Effect`]s a caller has to carry out. A timer that fires after its phase has
/// ended is ignored, which is how late answers and late timeouts are discarded.
///
/// ```
/// use std::time::Duration;
/// use maexchen::{Dealer, Effect, Event, Game, PlayerMessage, SimpleDice};
///
/// struct FixedDealer;
///
/// impl Dealer for FixedDealer {
///     fn roll(&mut self) -> SimpleDice {
///         SimpleDice::new(3, 1)
///     }
///
///     fn token(&mut self) -> String {
///         "token".to_string()
///     }
///
///     fn shuffle(&mut self, players: &mut [String]) {
///         players.sort();
///     }
/// }
///
/// let mut game = Game::new(FixedDealer, Duration::from_millis(250), true);
/// let anna = "127.0.0.1:4000".parse().unwrap();
/// let command = "REGISTER;anna".parse().unwrap();
///
/// let effects = game.handle(Event::Received { from: anna, command });
///
/// assert!(effects.contains(&Effect::Send { to: anna, message: PlayerMessage::Registered }));
/// ```
pub struct Game<D> {
    table: Table<D>,
    state: Option<Box<dyn State<D>>>,
}

impl<D: Dealer> Game<D> {
    /// Creates a game without any clients, played with the basic rules.
    ///
    /// Clients get `answer_timeout` to react to an invitation or a turn. With
    /// `start_rounds_early`, a round starts as soon as everybody has joined instead of
    /// after the timeout.
    #[must_use]
    pub fn new(dealer: D, answer_timeout: Duration, start_rounds_early: bool) -> Self {
        Self {
            table: Table::new(dealer, answer_timeout, start_rounds_early),
            state: Some(Box::new(lobby::Idle)),
        }
    }

    /// Plays with the given rule set instead of the basic rules.
    ///
    /// ```
    /// use std::time::Duration;
    /// use maexchen::{BlindRoll, Game, MiaRule, RandomDealer, RuleSet, Rules};
    ///
    /// let rules = Rules::new(MiaRule::Mia42, BlindRoll::Allowed);
    /// let game = Game::new(RandomDealer, Duration::from_millis(250), true)
    ///     .with_rule_set(RuleSet::Fixed(rules));
    /// ```
    #[must_use]
    pub fn with_rule_set(mut self, rule_set: RuleSet) -> Self {
        self.table.rule_set = rule_set;
        self
    }

    /// Applies the event and returns what the caller has to do about it.
    ///
    /// ```
    /// use std::time::Duration;
    /// use maexchen::{Dealer, Effect, Event, Game, PlayerMessage, RandomDealer};
    ///
    /// let mut game = Game::new(RandomDealer, Duration::from_millis(250), true);
    /// let anna = "127.0.0.1:4000".parse().unwrap();
    /// let command = "ECHO;hello".parse().unwrap();
    ///
    /// let effects = game.handle(Event::Received { from: anna, command });
    ///
    /// assert_eq!(
    ///     effects,
    ///     [Effect::Send { to: anna, message: PlayerMessage::Echoing("hello".to_string()) }]
    /// );
    /// ```
    pub fn handle(&mut self, event: Event) -> Vec<Effect> {
        match event {
            Event::Start => {
                if self.state.as_ref().is_some_and(|state| state.is_idle()) {
                    self.state = Some(lobby::new_round(&mut self.table));
                }
            }
            Event::Heartbeat => self.table.broadcast(&PlayerMessage::Heartbeat),
            Event::TimerFired(id) => self.transition(|state, table| state.on_timer(table, id)),
            Event::Received { from, command } => self.on_command(from, command),
        }
        self.table.take_effects()
    }

    fn transition(
        &mut self,
        step: impl FnOnce(Box<dyn State<D>>, &mut Table<D>) -> Box<dyn State<D>>,
    ) {
        if let Some(state) = self.state.take() {
            self.state = Some(step(state, &mut self.table));
        }
    }

    fn on_command(&mut self, from: SocketAddr, command: ClientCommand) {
        match command {
            ClientCommand::Echo(text) => {
                let text = if text.is_empty() {
                    "42".to_string()
                } else {
                    text
                };
                self.table.send_to_addr(from, PlayerMessage::Echoing(text));
            }
            ClientCommand::Register(name) => self.table.register(&name, from, Role::Player),
            ClientCommand::RegisterSpectator(name) => {
                self.table.register(&name, from, Role::Spectator);
            }
            ClientCommand::Unregister => self.unregister(from),
            command => {
                if let Some(name) = self.player_at(from) {
                    self.transition(|state, table| state.on_player_command(table, &name, command));
                }
            }
        }
    }

    fn player_at(&self, addr: SocketAddr) -> Option<String> {
        self.table
            .registry
            .name_at(addr)
            .map(str::to_string)
            .or_else(|| self.state.as_ref().and_then(|state| state.player_at(addr)))
    }

    fn unregister(&mut self, addr: SocketAddr) {
        if let Some(state) = &mut self.state {
            state.forget(addr);
        }
        if self.table.registry.unregister(addr) {
            self.table.send_to_addr(addr, PlayerMessage::Unregistered);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod properties;
