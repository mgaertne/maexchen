use core::{net::SocketAddr, time::Duration};

use proptest::prelude::*;

use super::{Effect, Event, Game};
use crate::{Dealer, PlayerMessage, SimpleDice};

struct FixedDealer;

impl Dealer for FixedDealer {
    fn roll(&mut self) -> SimpleDice {
        SimpleDice::new(3, 1)
    }

    fn token(&mut self) -> String {
        "t".to_string()
    }

    fn shuffle(&mut self, players: &mut [String]) {
        players.sort();
    }
}

const CLIENTS: usize = 3;

fn addr(client: usize) -> SocketAddr {
    SocketAddr::from((
        [127, 0, 0, 1],
        4000 + u16::try_from(client).unwrap_or_default(),
    ))
}

#[derive(Debug, Clone)]
enum Action {
    Command { client: usize, text: String },
    Start,
    Heartbeat,
    Timer(u64),
}

fn action() -> impl Strategy<Value = Action> {
    let client = 0..CLIENTS;
    let dice = (1u8..=6, 1u8..=6).prop_map(|(a, b)| format!("{a},{b}"));
    let command = prop_oneof![
        client
            .clone()
            .prop_map(|c| (c, format!("REGISTER;player{c}"))),
        client
            .clone()
            .prop_map(|c| (c, format!("REGISTER_SPECTATOR;spectator{c}"))),
        client.clone().prop_map(|c| (c, "UNREGISTER".to_string())),
        client.clone().prop_map(|c| (c, "JOIN;t".to_string())),
        client.clone().prop_map(|c| (c, "ROLL;t".to_string())),
        client.clone().prop_map(|c| (c, "BLIND_ROLL;t".to_string())),
        client.clone().prop_map(|c| (c, "SEE;t".to_string())),
        (client.clone(), dice).prop_map(|(c, d)| (c, format!("ANNOUNCE;{d};t"))),
        client.prop_map(|c| (c, "ECHO;x".to_string())),
    ];
    prop_oneof![
        8 => command.prop_map(|(client, text)| Action::Command { client, text }),
        1 => Just(Action::Start),
        1 => Just(Action::Heartbeat),
        3 => (1u64..40).prop_map(Action::Timer),
    ]
}

fn event(action: &Action) -> Option<Event> {
    match action {
        Action::Command { client, text } => Some(Event::Received {
            from: addr(*client),
            command: text.parse().ok()?,
        }),
        Action::Start => Some(Event::Start),
        Action::Heartbeat => Some(Event::Heartbeat),
        Action::Timer(id) => Some(Event::TimerFired(*id)),
    }
}

proptest! {
    #[test]
    fn any_sequence_of_events_keeps_the_game_consistent(
        actions in prop::collection::vec(action(), 0..120),
        start_rounds_early in any::<bool>(),
    ) {
        let mut game = Game::new(FixedDealer, Duration::from_millis(250), start_rounds_early);
        let mut newest_timer = 0;
        let mut newest_round = 0;

        for event in actions.iter().filter_map(event) {
            for effect in game.handle(event) {
                match effect {
                    Effect::StartTimer { id, .. } => {
                        prop_assert!(id > newest_timer, "timer {id} after {newest_timer}");
                        newest_timer = id;
                    }
                    Effect::Send { message: PlayerMessage::RoundStarted(round, _, _), .. } => {
                        prop_assert!(round >= newest_round, "round {round} after {newest_round}");
                        newest_round = round;
                    }
                    _ => {}
                }
            }
        }
    }
}
