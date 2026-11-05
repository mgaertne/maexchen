//! Benchmarks for the pure game logic and the wire format; no sockets involved.

use core::{net::SocketAddr, time::Duration};
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use maexchen::{ClientCommand, Dealer, Effect, Event, Game, PlayerMessage, SimpleDice};

struct FixedDealer;

impl Dealer for FixedDealer {
    fn roll(&mut self) -> SimpleDice {
        SimpleDice::new(3, 1)
    }

    fn token(&mut self) -> String {
        "token".to_string()
    }

    fn shuffle(&mut self, players: &mut [String]) {
        players.sort();
    }
}

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn command(text: &str) -> ClientCommand {
    text.parse().expect("the benchmark commands are valid")
}

fn send(game: &mut Game<FixedDealer>, from: SocketAddr, text: &str) -> Vec<Effect> {
    game.handle(Event::Received {
        from,
        command: command(text),
    })
}

fn play_round(players: u16) {
    let mut game = Game::new(FixedDealer, Duration::from_millis(250), true);
    for number in 0..players {
        send(
            &mut game,
            addr(9000 + number),
            &format!("REGISTER;player{number}"),
        );
    }
    game.handle(Event::Start);
    for number in 0..players {
        send(&mut game, addr(9000 + number), "JOIN;token");
    }
    for number in 0..players {
        let from = addr(9000 + number);
        send(&mut game, from, "ROLL;token");
        send(&mut game, from, "ANNOUNCE;3,1;token");
    }
    black_box(send(&mut game, addr(9000), "SEE;token"));
}

fn bench_round(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("round");
    for players in [2, 8, 32] {
        group.bench_function(format!("{players}_players"), |bencher| {
            bencher.iter(|| play_round(players));
        });
    }
    group.finish();
}

fn bench_wire_format(criterion: &mut Criterion) {
    criterion.bench_function("parse_announce", |bencher| {
        bencher.iter(|| black_box("ANNOUNCE;6,6;0a1b2c3d").parse::<ClientCommand>());
    });
    criterion.bench_function("format_player_lost", |bencher| {
        let message = PlayerMessage::ActualDice(SimpleDice::new(6, 6));
        bencher.iter(|| black_box(&message).to_string());
    });
}

criterion_group!(benches, bench_round, bench_wire_format);
criterion_main!(benches);
