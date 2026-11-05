#![no_main]

use core::{net::SocketAddr, time::Duration};

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use maexchen::{
    BlindRoll, Dealer, Effect, Event, Game, MiaRule, PlayerMessage, RuleSet, Rules, SimpleDice,
};

const CLIENTS: u8 = 3;

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

#[derive(Arbitrary, Debug)]
enum Step {
    Register(u8),
    RegisterSpectator(u8),
    Unregister(u8),
    Join(u8),
    Roll(u8),
    BlindRoll(u8),
    See(u8),
    Announce(u8, u8, u8),
    Echo(u8),
    Raw(u8, String),
    Start,
    Heartbeat,
    Timer(u8),
}

#[derive(Arbitrary, Debug)]
struct Input {
    start_rounds_early: bool,
    rule_set: u8,
    steps: Vec<Step>,
}

fn addr(client: u8) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 4000 + u16::from(client % CLIENTS)))
}

fn rule_set(choice: u8) -> RuleSet {
    let blind_roll = if choice & 1 == 0 {
        BlindRoll::Forbidden
    } else {
        BlindRoll::Allowed
    };
    match choice % 6 / 2 {
        0 => RuleSet::Fixed(Rules::new(MiaRule::Mia21, blind_roll)),
        1 => RuleSet::Fixed(Rules::new(MiaRule::Mia42, blind_roll)),
        _ => RuleSet::Random { blind_roll },
    }
}

fn event(step: Step) -> Option<Event> {
    let (client, text) = match step {
        Step::Register(c) => (c, format!("REGISTER;player{}", c % CLIENTS)),
        Step::RegisterSpectator(c) => (c, format!("REGISTER_SPECTATOR;spectator{}", c % CLIENTS)),
        Step::Unregister(c) => (c, "UNREGISTER".to_string()),
        Step::Join(c) => (c, "JOIN;t".to_string()),
        Step::Roll(c) => (c, "ROLL;t".to_string()),
        Step::BlindRoll(c) => (c, "BLIND_ROLL;t".to_string()),
        Step::See(c) => (c, "SEE;t".to_string()),
        Step::Announce(c, a, b) => (c, format!("ANNOUNCE;{},{};t", a % 6 + 1, b % 6 + 1)),
        Step::Echo(c) => (c, "ECHO;x".to_string()),
        Step::Raw(c, text) => (c, text),
        Step::Start => return Some(Event::Start),
        Step::Heartbeat => return Some(Event::Heartbeat),
        Step::Timer(id) => return Some(Event::TimerFired(u64::from(id % 40) + 1)),
    };
    Some(Event::Received {
        from: addr(client),
        command: text.parse().ok()?,
    })
}

fuzz_target!(|input: Input| {
    let mut game = Game::new(
        FixedDealer,
        Duration::from_millis(250),
        input.start_rounds_early,
    )
    .with_rule_set(rule_set(input.rule_set));
    let mut newest_timer = 0;
    let mut newest_round = 0;

    for event in input.steps.into_iter().filter_map(event) {
        for effect in game.handle(event) {
            match effect {
                Effect::StartTimer { id, .. } => {
                    assert!(id > newest_timer, "timer {id} after {newest_timer}");
                    newest_timer = id;
                }
                Effect::Send {
                    message: PlayerMessage::RoundStarted(round, _, _),
                    ..
                } => {
                    assert!(round >= newest_round, "round {round} after {newest_round}");
                    newest_round = round;
                }
                _ => {}
            }
        }
    }
});
