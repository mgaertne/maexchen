use core::net::SocketAddr;

use super::{
    TimerId, lobby,
    round::Round,
    state::{Next, State},
    table::Table,
};
use crate::{
    SimpleDice,
    dealer::Dealer,
    prelude::{ClientCommand, LosingReason, PlayerMessage},
};

/// How the dice that wait for an announcement were rolled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Roll {
    /// The player looked at the dice.
    Seen,
    /// The player did not look at the dice.
    Blind,
}

/// The current player must roll or look at the dice with the token of the turn.
struct AwaitingTurn {
    round: Round,
    timer: TimerId,
    token: String,
}

/// The current player rolled and must announce dice with the token of the roll.
struct AwaitingAnnouncement {
    round: Round,
    timer: TimerId,
    token: String,
    rolled: SimpleDice,
    second_roll_left: bool,
}

impl<D: Dealer> State<D> for AwaitingTurn {
    fn on_timer(self: Box<Self>, table: &mut Table<D>, id: TimerId) -> Next<D> {
        if id == self.timer {
            current_player_loses(table, &self.round, LosingReason::DidNotTakeTurn)
        } else {
            self
        }
    }

    fn on_player_command(
        self: Box<Self>,
        table: &mut Table<D>,
        name: &str,
        command: ClientCommand,
    ) -> Next<D> {
        if self.round.current_player() != name {
            return self;
        }
        match command {
            ClientCommand::Roll(token) if token == self.token => {
                roll_dice(table, self.round, Roll::Seen, true)
            }
            ClientCommand::BlindRoll(token)
                if token == self.token && self.round.rules.allows_blind_roll() =>
            {
                roll_dice(table, self.round, Roll::Blind, true)
            }
            ClientCommand::See(token) if token == self.token => show_dice(table, &self.round),
            _ => self,
        }
    }

    fn player_at(&self, addr: SocketAddr) -> Option<String> {
        self.round.player_at(addr).map(str::to_string)
    }

    fn forget(&mut self, addr: SocketAddr) {
        self.round.forget(addr);
    }
}

impl<D: Dealer> State<D> for AwaitingAnnouncement {
    fn on_timer(self: Box<Self>, table: &mut Table<D>, id: TimerId) -> Next<D> {
        if id == self.timer {
            current_player_loses(table, &self.round, LosingReason::DidNotAnnounce)
        } else {
            self
        }
    }

    fn on_player_command(
        self: Box<Self>,
        table: &mut Table<D>,
        name: &str,
        command: ClientCommand,
    ) -> Next<D> {
        if self.round.current_player() != name {
            return self;
        }
        match command {
            ClientCommand::Announce(dice, token) if token == self.token => {
                announce(table, self.round, self.rolled, dice)
            }
            ClientCommand::BlindRoll(token)
                if token == self.token
                    && self.second_roll_left
                    && self.round.rules.allows_blind_roll() =>
            {
                roll_dice(table, self.round, Roll::Blind, false)
            }
            _ => self,
        }
    }

    fn player_at(&self, addr: SocketAddr) -> Option<String> {
        self.round.player_at(addr).map(str::to_string)
    }

    fn forget(&mut self, addr: SocketAddr) {
        self.round.forget(addr);
    }
}

/// Asks the current player of the round for a turn.
pub(super) fn next_turn<D: Dealer>(table: &mut Table<D>, round: Round) -> Next<D> {
    let token = table.dealer.token();
    let timer = table.start_timer(table.answer_timeout);
    if let Some(to) = round.current_addr() {
        table.send_to_addr(to, PlayerMessage::YourTurn(token.clone()));
    }
    Box::new(AwaitingTurn {
        round,
        timer,
        token,
    })
}

fn roll_dice<D: Dealer>(
    table: &mut Table<D>,
    mut round: Round,
    roll: Roll,
    second_roll_left: bool,
) -> Next<D> {
    let current = round.current_player().to_string();
    let current_addr = round.current_addr();
    table.broadcast(&match roll {
        Roll::Seen => PlayerMessage::PlayerRolls(current),
        Roll::Blind => PlayerMessage::PlayerBlindRolls(current),
    });
    let rolled = table.dealer.roll();
    round.actual = Some(rolled);
    let token = table.dealer.token();
    let timer = table.start_timer(table.answer_timeout);
    if let Some(to) = current_addr {
        table.send_to_addr(
            to,
            match roll {
                Roll::Seen => PlayerMessage::YourRoll(rolled, token.clone()),
                Roll::Blind => PlayerMessage::YourBlindRoll(token.clone()),
            },
        );
    }
    Box::new(AwaitingAnnouncement {
        round,
        timer,
        token,
        rolled,
        second_roll_left,
    })
}

fn announce<D: Dealer>(
    table: &mut Table<D>,
    mut round: Round,
    rolled: SimpleDice,
    dice: SimpleDice,
) -> Next<D> {
    table.broadcast(&PlayerMessage::AnnouncedDiceBy(
        round.current_player().to_string(),
        dice,
    ));

    if round
        .announced
        .is_some_and(|announced| dice.cmp_under(announced, round.rules).is_le())
    {
        return current_player_loses(table, &round, LosingReason::AnnouncedLosingDice);
    }

    round.announced = Some(dice);
    if dice.is_mia(round.rules) {
        reveal_mia_announcement(table, &round, rolled)
    } else {
        round.advance();
        next_turn(table, round)
    }
}

fn reveal_mia_announcement<D: Dealer>(
    table: &mut Table<D>,
    round: &Round,
    rolled: SimpleDice,
) -> Next<D> {
    table.broadcast(&PlayerMessage::ActualDice(rolled));
    if rolled.is_mia(round.rules) {
        let losers = round
            .players
            .iter()
            .filter(|player| player.name != round.current_player())
            .map(|player| player.name.clone())
            .collect();
        players_lose(table, round, losers, LosingReason::Mia)
    } else {
        current_player_loses(table, round, LosingReason::LiedAboutMia)
    }
}

fn show_dice<D: Dealer>(table: &mut Table<D>, round: &Round) -> Next<D> {
    table.broadcast(&PlayerMessage::PlayerWantsToSee(
        round.current_player().to_string(),
    ));
    let (Some(actual), Some(announced)) = (round.actual, round.announced) else {
        return current_player_loses(table, round, LosingReason::SeeBeforeFirstRoll);
    };

    table.broadcast(&PlayerMessage::ActualDice(actual));
    if announced.cmp_under(actual, round.rules).is_gt() {
        let previous = round.previous_player().to_string();
        players_lose(table, round, vec![previous], LosingReason::CaughtBluffing)
    } else {
        current_player_loses(table, round, LosingReason::SeeFailed)
    }
}

fn current_player_loses<D: Dealer>(
    table: &mut Table<D>,
    round: &Round,
    reason: LosingReason,
) -> Next<D> {
    let current = round.current_player().to_string();
    players_lose(table, round, vec![current], reason)
}

fn players_lose<D: Dealer>(
    table: &mut Table<D>,
    round: &Round,
    mut losers: Vec<String>,
    reason: LosingReason,
) -> Next<D> {
    for player in round
        .players
        .iter()
        .filter(|player| !losers.contains(&player.name))
    {
        table.scores.increase(&player.name);
    }
    losers.sort();
    table.broadcast(&PlayerMessage::PlayerLost(losers, reason));
    table.broadcast_scores();
    lobby::new_round(table)
}
