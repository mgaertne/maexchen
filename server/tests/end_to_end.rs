//! Real clients of the `maexchen_bot` crate play a round against the server over UDP.

use core::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use std::sync::{Arc, Mutex};

use maexchen::{Game, RandomDealer, serve};
use maexchen_client::{
    bot::{Bot, Role, Server},
    client::{self, ClientError, Settings},
    simple_bot::SimpleBot,
    spectator_bot::SpectatorBot,
    transport::UdpTransport,
};
use tokio::{net::UdpSocket, task, time::timeout};

const ANSWER_TIMEOUT: Duration = Duration::from_millis(500);
const START_DELAY: Duration = Duration::from_millis(500);
const TEST_TIMEOUT: Duration = Duration::from_secs(15);

type Log = Arc<Mutex<Vec<String>>>;

/// Remembers the commands a bot receives and stops it once a round is over.
struct Recorder<B> {
    bot: B,
    log: Log,
    stop: Arc<AtomicBool>,
}

impl<B: Bot> Bot for Recorder<B> {
    fn role(&self) -> Role {
        self.bot.role()
    }

    fn on_message(&mut self, server: &Server, command: &str, parameters: &[&str]) {
        self.log.lock().unwrap().push(command.to_string());
        if command == "PLAYER LOST" {
            self.stop.store(true, Ordering::Release);
        }
        self.bot.on_message(server, command, parameters);
    }
}

fn settings(name: &str) -> Settings {
    Settings {
        name: name.to_string(),
        auto_join: true,
        heartbeat_timeout: None,
        answer_timeout: Duration::from_secs(2),
    }
}

fn start_client<B: Bot + Send + 'static>(
    port: u16,
    name: &'static str,
    bot: B,
) -> (Log, task::JoinHandle<Result<(), ClientError>>) {
    let log = Log::default();
    let stop = Arc::new(AtomicBool::new(false));
    let mut recorder = Recorder {
        bot,
        log: Arc::clone(&log),
        stop: Arc::clone(&stop),
    };
    let handle = task::spawn_blocking(move || {
        let transport = UdpTransport::connect("127.0.0.1", port)?;
        client::run(&transport, &mut recorder, &settings(name), &stop)
    });
    (log, handle)
}

fn commands(log: &Log) -> Vec<String> {
    log.lock().unwrap().clone()
}

#[tokio::test]
async fn two_players_and_a_spectator_get_through_a_round() {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    let game = Game::new(RandomDealer, ANSWER_TIMEOUT, true);
    let server = tokio::spawn(serve(socket, game, START_DELAY));

    let (anna, anna_done) = start_client(port, "anna", SimpleBot);
    let (bert, bert_done) = start_client(port, "bert", SimpleBot);
    let (zoe, zoe_done) = start_client(port, "zoe", SpectatorBot);

    for done in [anna_done, bert_done, zoe_done] {
        timeout(TEST_TIMEOUT, done)
            .await
            .expect("the clients finish in time")
            .unwrap()
            .expect("the client plays without an error");
    }
    server.abort();

    for player in [&anna, &bert] {
        let seen = commands(player);
        assert!(seen.contains(&"ROUND STARTED".to_string()), "{seen:?}");
        assert!(seen.contains(&"PLAYER LOST".to_string()), "{seen:?}");
    }
    let turns = [&anna, &bert]
        .iter()
        .filter(|player| commands(player).contains(&"YOUR TURN".to_string()))
        .count();
    assert!(turns >= 1, "somebody had a turn");

    let watched = commands(&zoe);
    assert!(
        watched.contains(&"ROUND STARTED".to_string()),
        "{watched:?}"
    );
    assert!(watched.contains(&"PLAYER LOST".to_string()), "{watched:?}");
    assert!(!watched.contains(&"YOUR TURN".to_string()), "{watched:?}");
    assert!(
        !watched.contains(&"ROUND STARTING".to_string()),
        "{watched:?}"
    );
}
