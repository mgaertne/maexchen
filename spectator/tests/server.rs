//! The spectator against the real server: the wire format, a round and a reconnection.

use core::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use std::{
    net::UdpSocket as StdSocket,
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    thread,
};

use maexchen::{
    CancellationReason, Game, LosingReason, PlayerMessage, RandomDealer, SimpleDice, serve,
};
use maexchen_client::{
    client::{self, Settings},
    simple_bot::SimpleBot,
    transport::UdpTransport,
};
use maexchen_spectator_core::{
    connection::{self, Endpoint},
    message::{CancelReason, Dice, LossReason, Message, parse},
    update::{Status, Update},
};
use tokio::{net::UdpSocket, task};

const ANSWER_TIMEOUT: Duration = Duration::from_millis(500);
const START_DELAY: Duration = Duration::from_millis(500);
const WAIT: Duration = Duration::from_secs(20);

fn parse_wire(message: &PlayerMessage) -> Option<Message> {
    let text = message.to_string();
    let mut parts = text.split(';');
    let command = parts.next()?;
    let parameters: Vec<&str> = parts.collect();
    parse(command, &parameters)
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

#[test]
fn every_message_for_spectators_is_understood() {
    let dice = SimpleDice::new(1, 6);
    let expected = Dice { high: 6, low: 1 };
    let cases = [
        (
            PlayerMessage::RoundStarted(7, None, names(&["bert", "anna"])),
            Message::RoundStarted {
                number: 7,
                players: names(&["bert", "anna"]),
                rules: None,
            },
        ),
        (
            PlayerMessage::RoundCanceled(CancellationReason::OnlyOnePlayer),
            Message::RoundCanceled(CancelReason::OnlyOnePlayer),
        ),
        (
            PlayerMessage::PlayerRolls("anna".to_string()),
            Message::PlayerRolls("anna".to_string()),
        ),
        (
            PlayerMessage::PlayerBlindRolls("anna".to_string()),
            Message::PlayerBlindRolls("anna".to_string()),
        ),
        (
            PlayerMessage::PlayerWantsToSee("anna".to_string()),
            Message::PlayerWantsToSee("anna".to_string()),
        ),
        (
            PlayerMessage::AnnouncedDiceBy("anna".to_string(), dice),
            Message::Announced {
                player: "anna".to_string(),
                dice: expected,
            },
        ),
        (
            PlayerMessage::ActualDice(dice),
            Message::ActualDice(expected),
        ),
        (
            PlayerMessage::PlayerLost(names(&["anna", "bert"]), LosingReason::Mia),
            Message::PlayerLost {
                players: names(&["anna", "bert"]),
                reason: LossReason::Mia,
            },
        ),
        (
            PlayerMessage::CurrentScore(vec![("anna".to_string(), 3), ("bert".to_string(), -1)]),
            Message::Score(vec![("anna".to_string(), 3), ("bert".to_string(), -1)]),
        ),
    ];

    for (sent, understood) in cases {
        assert_eq!(parse_wire(&sent), Some(understood), "{sent}");
    }
}

#[test]
fn messages_for_players_only_are_ignored() {
    let ignored = [
        PlayerMessage::Registered,
        PlayerMessage::Unregistered,
        PlayerMessage::Heartbeat,
        PlayerMessage::RoundStarting("token".to_string()),
        PlayerMessage::YourTurn("token".to_string()),
        PlayerMessage::YourRoll(SimpleDice::new(1, 2), "token".to_string()),
        PlayerMessage::YourBlindRoll("token".to_string()),
    ];

    for message in ignored {
        assert_eq!(parse_wire(&message), None, "{message}");
    }
}

#[test]
fn every_reason_of_the_server_is_known() {
    let losing = [
        LosingReason::SeeBeforeFirstRoll,
        LosingReason::LiedAboutMia,
        LosingReason::AnnouncedLosingDice,
        LosingReason::DidNotAnnounce,
        LosingReason::DidNotTakeTurn,
        LosingReason::InvalidTurn,
        LosingReason::SeeFailed,
        LosingReason::CaughtBluffing,
        LosingReason::Mia,
    ];
    for reason in losing {
        let known = LossReason::from(reason.to_string().as_str());
        assert!(!matches!(known, LossReason::Other(_)), "{reason}");
    }

    let canceled = [
        CancellationReason::NoPlayers,
        CancellationReason::OnlyOnePlayer,
    ];
    for reason in canceled {
        let known = CancelReason::from(reason.to_string().as_str());
        assert!(!matches!(known, CancelReason::Other(_)), "{reason}");
    }
}

fn start_server(socket: UdpSocket) -> task::JoinHandle<std::io::Result<Game<RandomDealer>>> {
    let game = Game::new(RandomDealer, ANSWER_TIMEOUT, true);
    tokio::spawn(serve(socket, game, START_DELAY))
}

fn start_player(port: u16, name: &'static str, stop: Arc<AtomicBool>) -> task::JoinHandle<()> {
    task::spawn_blocking(move || {
        let transport = UdpTransport::connect("127.0.0.1", port).expect("a local socket");
        let settings = Settings {
            name: name.to_string(),
            auto_join: true,
            heartbeat_timeout: None,
            answer_timeout: Duration::from_secs(2),
        };
        client::run(&transport, &mut SimpleBot, &settings, &stop).expect("the player plays");
    })
}

fn endpoint(port: u16) -> Endpoint {
    Endpoint {
        host: "127.0.0.1".to_string(),
        port,
        name: "zoe".to_string(),
    }
}

fn wait_for(updates: &Receiver<Update>, mut wanted: impl FnMut(&Update) -> bool) -> bool {
    let deadline = std::time::Instant::now() + WAIT;
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match updates.recv_timeout(left) {
            Ok(update) if wanted(&update) => return true,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    false
}

async fn play_a_round(port: u16, updates: &Receiver<Update>) {
    let stop = Arc::new(AtomicBool::new(false));
    let anna = start_player(port, "anna", Arc::clone(&stop));
    let bert = start_player(port, "bert", Arc::clone(&stop));

    let lost = wait_for(updates, |update| {
        matches!(update, Update::Message(Message::PlayerLost { .. }))
    });
    stop.store(true, Ordering::Release);
    anna.await.unwrap();
    bert.await.unwrap();
    assert!(lost, "the spectator saw the round end");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_spectator_follows_a_round() {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    let server = start_server(socket);

    let (sender, updates) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let spectator = connection::spawn(endpoint(port), sender, Arc::clone(&stop));

    play_a_round(port, &updates).await;

    stop.store(true, Ordering::Release);
    spectator.join().expect("the connection ends cleanly");
    server.abort();

    let seen: Vec<Update> = updates.try_iter().collect();
    assert!(
        seen.iter()
            .any(|update| matches!(update, Update::Message(Message::Score(_)))),
        "{seen:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_spectator_connects_again_when_the_server_comes_back() {
    let port = StdSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let (sender, updates) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let spectator = connection::spawn(endpoint(port), sender, Arc::clone(&stop));

    assert!(
        wait_for(&updates, |update| matches!(
            update,
            Update::Status(Status::Reconnecting { .. })
        )),
        "the spectator notices that nobody answers"
    );

    thread::sleep(Duration::from_millis(100));
    let socket = UdpSocket::bind(("127.0.0.1", port)).await.unwrap();
    let server = start_server(socket);
    play_a_round(port, &updates).await;

    stop.store(true, Ordering::Release);
    spectator.join().expect("the connection ends cleanly");
    server.abort();
}

mod random_rules {
    use maexchen::{BlindRoll, Dealer, MiaRule, RuleSet, Rules};
    use maexchen_spectator_core::message::AnnouncedRules;

    use super::*;

    struct AlternatingRules {
        deal: RandomDealer,
        round: u32,
    }

    impl Dealer for AlternatingRules {
        fn roll(&mut self) -> SimpleDice {
            self.deal.roll()
        }

        fn token(&mut self) -> String {
            self.deal.token()
        }

        fn shuffle(&mut self, players: &mut [String]) {
            self.deal.shuffle(players);
        }

        fn rules(&mut self, _rule_set: RuleSet) -> Rules {
            self.round += 1;
            if self.round % 2 == 1 {
                Rules::new(MiaRule::Mia42, BlindRoll::Allowed)
            } else {
                Rules::new(MiaRule::Mia21, BlindRoll::Forbidden)
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_spectator_learns_the_rules_of_every_round() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = socket.local_addr().unwrap().port();
        let dealer = AlternatingRules {
            deal: RandomDealer,
            round: 0,
        };
        let game = Game::new(dealer, ANSWER_TIMEOUT, true).with_rule_set(RuleSet::Random {
            blind_roll: BlindRoll::Allowed,
        });
        let server = tokio::spawn(serve(socket, game, START_DELAY));

        let (sender, updates) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let spectator = connection::spawn(endpoint(port), sender, Arc::clone(&stop));
        let players_stop = Arc::new(AtomicBool::new(false));
        let anna = start_player(port, "anna", Arc::clone(&players_stop));
        let bert = start_player(port, "bert", Arc::clone(&players_stop));

        let mut announced = Vec::new();
        let seen_two_rounds = wait_for(&updates, |update| {
            if let Update::Message(Message::RoundStarted { rules, .. }) = update {
                announced.push(*rules);
            }
            announced.len() >= 2
        });

        players_stop.store(true, Ordering::Release);
        anna.await.unwrap();
        bert.await.unwrap();
        stop.store(true, Ordering::Release);
        spectator.join().expect("the connection ends cleanly");
        server.abort();

        assert!(seen_two_rounds, "{announced:?}");
        assert_eq!(
            announced[..2],
            [
                Some(AnnouncedRules {
                    half_mia: true,
                    blind_roll: true
                }),
                Some(AnnouncedRules {
                    half_mia: false,
                    blind_roll: false
                }),
            ]
        );
    }
}
