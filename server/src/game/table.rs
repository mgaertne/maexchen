use core::{net::SocketAddr, time::Duration};

use super::{Effect, TimerId};
use crate::{
    RuleSet,
    dealer::Dealer,
    prelude::PlayerMessage,
    registry::{Registry, Role},
    score::Score,
};

/// Everything the phases of a game share: who is registered, the scores, the dealer and the
/// effects collected for the caller.
pub(super) struct Table<D> {
    pub(super) dealer: D,
    pub(super) answer_timeout: Duration,
    pub(super) start_rounds_early: bool,
    pub(super) rule_set: RuleSet,
    pub(super) registry: Registry,
    pub(super) scores: Score,
    pub(super) round_number: u64,
    last_timer: TimerId,
    outbox: Vec<Effect>,
}

impl<D: Dealer> Table<D> {
    pub(super) fn new(dealer: D, answer_timeout: Duration, start_rounds_early: bool) -> Self {
        Self {
            dealer,
            answer_timeout,
            start_rounds_early,
            rule_set: RuleSet::default(),
            registry: Registry::default(),
            scores: Score::default(),
            round_number: 0,
            last_timer: 0,
            outbox: vec![],
        }
    }

    pub(super) fn take_effects(&mut self) -> Vec<Effect> {
        core::mem::take(&mut self.outbox)
    }

    pub(super) fn register(&mut self, name: &str, addr: SocketAddr, role: Role) {
        if let Err(reason) = self.registry.register(name, addr, role) {
            self.send_to_addr(addr, PlayerMessage::RegistrationRejected(reason));
            return;
        }
        if role == Role::Player {
            self.scores.register(name);
            self.send_scores_to_spectators();
        }
        self.send_to_addr(addr, PlayerMessage::Registered);
    }

    pub(super) fn start_timer(&mut self, after: Duration) -> TimerId {
        self.last_timer += 1;
        let id = self.last_timer;
        self.outbox.push(Effect::StartTimer { id, after });
        id
    }

    pub(super) fn send_to_addr(&mut self, to: SocketAddr, message: PlayerMessage) {
        self.outbox.push(Effect::Send { to, message });
    }

    pub(super) fn send_to_name(&mut self, name: &str, message: PlayerMessage) {
        if let Some(to) = self.registry.addr_of(name) {
            self.send_to_addr(to, message);
        }
    }

    pub(super) fn broadcast(&mut self, message: &PlayerMessage) {
        let recipients: Vec<SocketAddr> = self.registry.addrs().collect();
        for to in recipients {
            self.send_to_addr(to, message.clone());
        }
    }

    pub(super) fn broadcast_scores(&mut self) {
        let message = PlayerMessage::CurrentScore(self.scores.standings());
        self.broadcast(&message);
    }

    fn send_scores_to_spectators(&mut self) {
        let message = PlayerMessage::CurrentScore(self.scores.standings());
        let spectators: Vec<SocketAddr> = self.registry.spectator_addrs().collect();
        for to in spectators {
            self.send_to_addr(to, message.clone());
        }
    }
}
