use core::net::SocketAddr;

use crate::{Rules, SimpleDice};

#[derive(Debug)]
pub(super) struct Participant {
    pub(super) name: String,
    pub(super) addr: Option<SocketAddr>,
}

#[derive(Debug)]
pub(super) struct Round {
    pub(super) players: Vec<Participant>,
    current: usize,
    pub(super) actual: Option<SimpleDice>,
    pub(super) announced: Option<SimpleDice>,
    pub(super) rules: Rules,
}

impl Round {
    pub(super) fn new(players: Vec<Participant>, rules: Rules) -> Self {
        Self {
            players,
            current: 0,
            actual: None,
            announced: None,
            rules,
        }
    }

    pub(super) fn current_player(&self) -> &str {
        &self.players[self.current].name
    }

    pub(super) fn current_addr(&self) -> Option<SocketAddr> {
        self.players[self.current].addr
    }

    pub(super) fn player_at(&self, addr: SocketAddr) -> Option<&str> {
        self.players
            .iter()
            .find(|player| player.addr == Some(addr))
            .map(|player| player.name.as_str())
    }

    pub(super) fn forget(&mut self, addr: SocketAddr) {
        for player in &mut self.players {
            if player.addr == Some(addr) {
                player.addr = None;
            }
        }
    }

    pub(super) fn previous_player(&self) -> &str {
        &self.players[(self.current + self.players.len() - 1) % self.players.len()].name
    }

    pub(super) fn advance(&mut self) {
        self.current = (self.current + 1) % self.players.len();
    }
}
