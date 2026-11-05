use core::time::Duration;

use super::{
    TimerId,
    round::{Participant, Round},
    state::{Next, State},
    table::Table,
    turns,
};
use crate::{
    dealer::Dealer,
    prelude::{CancellationReason, ClientCommand, PlayerMessage},
};

const NEW_ROUND_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Nothing has started yet.
pub(super) struct Idle;

/// Too few players are registered for a round; asks again after a short while.
struct WaitingForPlayers {
    timer: TimerId,
}

/// The players are invited and answer with the token of their invitation.
struct Joining {
    timer: TimerId,
    invitations: Vec<(String, String)>,
    joined: Vec<String>,
}

impl<D: Dealer> State<D> for Idle {
    fn on_timer(self: Box<Self>, _table: &mut Table<D>, _id: TimerId) -> Next<D> {
        self
    }

    fn on_player_command(
        self: Box<Self>,
        _table: &mut Table<D>,
        _name: &str,
        _command: ClientCommand,
    ) -> Next<D> {
        self
    }

    fn is_idle(&self) -> bool {
        true
    }
}

impl<D: Dealer> State<D> for WaitingForPlayers {
    fn on_timer(self: Box<Self>, table: &mut Table<D>, id: TimerId) -> Next<D> {
        if id == self.timer {
            new_round(table)
        } else {
            self
        }
    }

    fn on_player_command(
        self: Box<Self>,
        _table: &mut Table<D>,
        _name: &str,
        _command: ClientCommand,
    ) -> Next<D> {
        self
    }
}

impl<D: Dealer> State<D> for Joining {
    fn on_timer(self: Box<Self>, table: &mut Table<D>, id: TimerId) -> Next<D> {
        if id != self.timer {
            return self;
        }
        if self.joined.is_empty() {
            cancel_round(table, CancellationReason::NoPlayers)
        } else {
            start_round(table, self.joined)
        }
    }

    fn on_player_command(
        mut self: Box<Self>,
        table: &mut Table<D>,
        name: &str,
        command: ClientCommand,
    ) -> Next<D> {
        let invited = matches!(
            &command,
            ClientCommand::Join(token)
                if self.invitations.iter().any(|(n, t)| n == name && t == token)
        );
        if invited && !self.joined.iter().any(|joined| joined == name) {
            self.joined.push(name.to_string());
            if table.start_rounds_early && self.joined.len() == table.registry.player_count() {
                return start_round(table, self.joined);
            }
        }
        self
    }
}

/// Invites all registered players, or waits for more of them.
pub(super) fn new_round<D: Dealer>(table: &mut Table<D>) -> Next<D> {
    if table.registry.player_count() < 2 {
        let timer = table.start_timer(NEW_ROUND_RETRY_DELAY);
        return Box::new(WaitingForPlayers { timer });
    }

    table.round_number += 1;
    let players: Vec<String> = table.registry.player_names().map(str::to_string).collect();
    let invitations: Vec<(String, String)> = players
        .into_iter()
        .map(|name| (name, table.dealer.token()))
        .collect();
    for (name, token) in &invitations {
        table.send_to_name(name, PlayerMessage::RoundStarting(token.clone()));
    }
    let timer = table.start_timer(table.answer_timeout);
    Box::new(Joining {
        timer,
        invitations,
        joined: vec![],
    })
}

fn start_round<D: Dealer>(table: &mut Table<D>, mut players: Vec<String>) -> Next<D> {
    players.retain(|player| table.registry.addr_of(player).is_some());
    table.dealer.shuffle(&mut players);
    let rules = table.dealer.rules(table.rule_set);
    let announced = table.rule_set.announces_rules().then_some(rules);
    table.broadcast(&PlayerMessage::RoundStarted(
        table.round_number,
        announced,
        players.clone(),
    ));
    if players.len() <= 1 {
        return cancel_round(table, CancellationReason::OnlyOnePlayer);
    }
    let participants = players
        .into_iter()
        .map(|name| Participant {
            addr: table.registry.addr_of(&name),
            name,
        })
        .collect();
    turns::next_turn(table, Round::new(participants, rules))
}

fn cancel_round<D: Dealer>(table: &mut Table<D>, reason: CancellationReason) -> Next<D> {
    table.broadcast(&PlayerMessage::RoundCanceled(reason));
    table.broadcast_scores();
    new_round(table)
}
