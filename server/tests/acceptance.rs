//! Port of `server/spec/AcceptanceSpec.coffee`: fake clients talk UDP to a running server.

use core::{net::SocketAddr, time::Duration};
use std::sync::{Arc, Mutex};

use maexchen::{BlindRoll, Dealer, Game, MiaRule, RuleSet, Rules, SimpleDice, serve};
use tokio::{net::UdpSocket, task::JoinHandle, time::timeout};

const ANSWER_TIMEOUT: Duration = Duration::from_millis(100);
const START_DELAY: Duration = Duration::from_millis(100);
const RECEIVE_TIMEOUT: Duration = Duration::from_secs(2);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(5);

fn dice(die1: u8, die2: u8) -> SimpleDice {
    SimpleDice::new(die1, die2)
}

fn mia() -> SimpleDice {
    dice(2, 1)
}

/// Always rolls the dice the test has set and orders players alphabetically.
struct RiggedDealer {
    dice: Arc<Mutex<SimpleDice>>,
    tokens_issued: u64,
}

impl Dealer for RiggedDealer {
    fn roll(&mut self) -> SimpleDice {
        *self
            .dice
            .lock()
            .expect("the test thread does not panic while holding the lock")
    }

    fn token(&mut self) -> String {
        self.tokens_issued += 1;
        format!("token-{}", self.tokens_issued)
    }

    fn shuffle(&mut self, players: &mut [String]) {
        players.sort();
    }
}

struct TestServer {
    addr: SocketAddr,
    rolled_dice: Arc<Mutex<SimpleDice>>,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestServer {
    async fn start() -> Self {
        Self::start_with(RuleSet::default()).await
    }

    async fn start_with(rule_set: RuleSet) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("a free local UDP port is available");
        let addr = socket.local_addr().expect("a bound socket has an address");
        let rolled_dice = Arc::new(Mutex::new(dice(1, 1)));
        let dealer = RiggedDealer {
            dice: Arc::clone(&rolled_dice),
            tokens_issued: 0,
        };
        let game = Game::new(dealer, ANSWER_TIMEOUT, true).with_rule_set(rule_set);
        let task = tokio::spawn(async move {
            let _ = serve(socket, game, START_DELAY).await;
        });
        Self {
            addr,
            rolled_dice,
            task,
        }
    }

    fn server_rolls(&self, rolled: SimpleDice) {
        *self
            .rolled_dice
            .lock()
            .expect("the test thread does not panic while holding the lock") = rolled;
    }

    async fn client(&self, name: &str) -> FakeClient {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("a free local UDP port is available");
        FakeClient {
            name: name.to_string(),
            socket,
            server: self.addr,
            token: "noTokenReceived".to_string(),
            history: vec![],
        }
    }

    async fn player(&self, name: &str) -> FakeClient {
        let mut client = self.client(name).await;
        client.send(&format!("REGISTER;{name}")).await;
        client.receives("REGISTERED").await;
        client
    }

    async fn spectator(&self, name: &str) -> FakeClient {
        let mut client = self.client(name).await;
        client.send(&format!("REGISTER_SPECTATOR;{name}")).await;
        client.receives("REGISTERED").await;
        client
    }

    async fn players(&self, names: &[&str]) -> Vec<FakeClient> {
        let mut players = vec![];
        for name in names {
            players.push(self.player(name).await);
        }
        players
    }
}

struct FakeClient {
    name: String,
    socket: UdpSocket,
    server: SocketAddr,
    token: String,
    history: Vec<String>,
}

macro_rules! each {
    ($clients:expr, $method:ident($($argument:expr),*)) => {
        for client in $clients.iter_mut() {
            client.$method($($argument),*).await;
        }
    };
}

impl FakeClient {
    async fn send(&self, message: &str) {
        self.socket
            .send_to(message.as_bytes(), self.server)
            .await
            .expect("local UDP send succeeds");
    }

    async fn next_message(&mut self) -> Option<String> {
        self.next_message_within(RECEIVE_TIMEOUT).await
    }

    async fn next_message_within(&mut self, limit: Duration) -> Option<String> {
        let mut buffer = [0u8; 2048];
        let (size, _) = timeout(limit, self.socket.recv_from(&mut buffer))
            .await
            .ok()?
            .expect("local UDP receive succeeds");
        let message = String::from_utf8_lossy(&buffer[..size]).into_owned();
        self.history.push(message.clone());
        Some(message)
    }

    async fn receives(&mut self, expected: &str) {
        while let Some(message) = self.next_message().await {
            if message == expected {
                return;
            }
        }
        panic!(
            "[{}] did not receive {expected:?}, only {:?}",
            self.name, self.history
        );
    }

    async fn receives_a_heartbeat(&mut self) {
        while let Some(message) = self.next_message_within(HEARTBEAT_TIMEOUT).await {
            if message == "HEARTBEAT" {
                return;
            }
        }
        panic!("[{}] did not receive a heartbeat", self.name);
    }

    async fn receives_with_token(&mut self, prefix: &str) {
        let wanted = format!("{prefix};");
        while let Some(message) = self.next_message().await {
            if let Some(token) = message.strip_prefix(&wanted) {
                self.token = token.to_string();
                return;
            }
        }
        panic!(
            "[{}] did not receive {prefix:?} with a token, only {:?}",
            self.name, self.history
        );
    }

    fn did_not_receive_offer_to_join_round(&mut self) {
        let mut buffer = [0u8; 2048];
        while let Ok((size, _)) = self.socket.try_recv_from(&mut buffer) {
            self.history
                .push(String::from_utf8_lossy(&buffer[..size]).into_owned());
        }
        assert!(
            !self
                .history
                .iter()
                .any(|message| message.starts_with("ROUND STARTING")),
            "[{}] was invited: {:?}",
            self.name,
            self.history
        );
    }

    async fn receives_offer_to_join_round(&mut self) {
        self.receives_with_token("ROUND STARTING").await;
    }

    async fn joins_round(&mut self) {
        self.send(&format!("JOIN;{}", self.token)).await;
    }

    async fn receives_notification_that_round_was_canceled(&mut self, reason: &str) {
        self.receives(&format!("ROUND CANCELED;{reason}")).await;
    }

    async fn receives_notification_that_round_is_starting(&mut self, round: u64, players: &[&str]) {
        self.receives(&format!("ROUND STARTED;{round};{}", players.join(",")))
            .await;
    }

    async fn is_asked_to_play_a_turn(&mut self) {
        self.receives_with_token("YOUR TURN").await;
    }

    async fn rolls(&mut self) {
        self.send(&format!("ROLL;{}", self.token)).await;
    }

    async fn wants_to_see(&mut self) {
        self.send(&format!("SEE;{}", self.token)).await;
    }

    async fn receives_rolled_dice(&mut self, rolled: SimpleDice) {
        self.receives_with_token(&format!("ROLLED;{rolled}")).await;
    }

    async fn announces_dice(&mut self, announced: SimpleDice) {
        self.send(&format!("ANNOUNCE;{announced};{}", self.token))
            .await;
    }

    async fn receives_dice_announcement(&mut self, player: &str, announced: SimpleDice) {
        self.receives(&format!("ANNOUNCED;{player};{announced}"))
            .await;
    }

    async fn receives_actual_dice(&mut self, actual: SimpleDice) {
        self.receives(&format!("ACTUAL DICE;{actual}")).await;
    }

    async fn receives_notification_that_player_rolls(&mut self, player: &str) {
        self.receives(&format!("PLAYER ROLLS;{player}")).await;
    }

    async fn receives_notification_that_player_wants_to_see(&mut self, player: &str) {
        self.receives(&format!("PLAYER WANTS TO SEE;{player}"))
            .await;
    }

    async fn receives_notification_that_players_lost(&mut self, players: &[&str], reason: &str) {
        self.receives(&format!("PLAYER LOST;{};{reason}", players.join(",")))
            .await;
    }

    async fn receives_notification_that_player_lost(&mut self, player: &str, reason: &str) {
        self.receives_notification_that_players_lost(&[player], reason)
            .await;
    }

    async fn receives_scores(&mut self, scores: &[(&str, i32)]) {
        let scores: Vec<String> = scores
            .iter()
            .map(|(name, score)| format!("{name}:{score}"))
            .collect();
        self.receives(&format!("SCORE;{}", scores.join(","))).await;
    }

    async fn blind_rolls(&mut self) {
        self.send(&format!("BLIND_ROLL;{}", self.token)).await;
    }

    async fn receives_notification_that_player_blind_rolls(&mut self, player: &str) {
        self.receives(&format!("PLAYER BLIND ROLLS;{player}")).await;
    }

    async fn receives_blind_rolled_confirmation(&mut self) {
        self.receives_with_token("BLIND_ROLLED").await;
    }
}

mod echo_service {
    use super::*;

    #[tokio::test]
    async fn echoes_the_argument_of_an_echo_request() {
        let server = TestServer::start().await;
        let mut client = server.client("client").await;

        client.send("ECHO;hello").await;
        client.receives("ECHOING;hello").await;
        client.send("ECHO;a longish text with 33 characters").await;
        client
            .receives("ECHOING;a longish text with 33 characters")
            .await;
    }

    #[tokio::test]
    async fn concatenates_the_arguments_of_an_echo_request() {
        let server = TestServer::start().await;
        let mut client = server.client("client").await;

        client.send("ECHO;hello;you").await;
        client.receives("ECHOING;helloyou").await;
    }

    #[tokio::test]
    async fn ignores_unknown_commands() {
        let server = TestServer::start().await;
        let mut client = server.client("client").await;

        client.send("NONSENSE").await;
        client.send("ECHO;still alive").await;

        client.receives("ECHOING;still alive").await;
    }

    #[tokio::test]
    async fn echoes_42_if_no_argument_is_given() {
        let server = TestServer::start().await;
        let mut client = server.client("client").await;

        client.send("ECHO").await;
        client.receives("ECHOING;42").await;
    }
}

mod player_setup {
    use super::*;

    #[tokio::test]
    async fn accepts_player_registrations() {
        let server = TestServer::start().await;
        let mut client = server.client("client").await;

        client.send("REGISTER;client").await;
        client.receives("REGISTERED").await;
    }

    #[tokio::test]
    async fn accepts_player_unregistration() {
        let server = TestServer::start().await;
        let mut client = server.client("client").await;

        client.send("REGISTER;client").await;
        client.send("UNREGISTER").await;
        client.receives("UNREGISTERED").await;
    }
}

mod round_setup {
    use super::*;

    #[tokio::test]
    async fn keeps_trying_to_start_a_round_while_nobody_joins() {
        let server = TestServer::start().await;
        let mut players = server.players(&["testClient1", "testClient2"]).await;

        for player in &mut players {
            player.receives_offer_to_join_round().await;
            player
                .receives_notification_that_round_was_canceled("NO_PLAYERS")
                .await;
        }
        players[0].receives_offer_to_join_round().await;
        players[0]
            .receives_notification_that_round_was_canceled("NO_PLAYERS")
            .await;
    }

    #[tokio::test]
    async fn does_not_invite_spectators_to_join_rounds() {
        let server = TestServer::start().await;
        let mut spectator = server.spectator("theSpectator").await;
        let mut players = server.players(&["thePlayer1", "thePlayer2"]).await;

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());
        each!(
            players,
            receives_notification_that_round_is_starting(1, &["thePlayer1", "thePlayer2"])
        );

        spectator
            .receives_notification_that_round_is_starting(1, &["thePlayer1", "thePlayer2"])
            .await;
        spectator.did_not_receive_offer_to_join_round();
    }

    #[tokio::test]
    async fn cancels_the_round_if_only_one_player_joins() {
        let server = TestServer::start().await;
        let mut players = server.players(&["thePlayer1", "thePlayer2"]).await;

        each!(players, receives_offer_to_join_round());
        players[0].joins_round().await;
        players[0]
            .receives_notification_that_round_is_starting(1, &["thePlayer1"])
            .await;
        players[0]
            .receives_notification_that_round_was_canceled("ONLY_ONE_PLAYER")
            .await;

        players[0].receives_offer_to_join_round().await;
    }
}

mod previously_registered_player_registers_again {
    use super::*;

    async fn play_round(player1: &mut FakeClient, player2: &mut FakeClient) {
        player1.is_asked_to_play_a_turn().await;
        player1.rolls().await;
        player1.receives_rolled_dice(dice(3, 1)).await;
        player1.announces_dice(dice(6, 6)).await;
        player2.is_asked_to_play_a_turn().await;
        player2.wants_to_see().await;
    }

    #[tokio::test]
    async fn the_new_player_takes_the_place_of_the_old_one_in_the_next_round_keeping_the_score() {
        let server = TestServer::start().await;
        let mut old_player = server.player("thePlayer").await;
        let mut other_player = server.player("theOtherPlayer").await;
        server.server_rolls(dice(3, 1));

        other_player.receives_offer_to_join_round().await;
        other_player.joins_round().await;
        old_player.receives_offer_to_join_round().await;
        old_player.joins_round().await;

        let mut new_player = server.player("thePlayer").await;

        play_round(&mut other_player, &mut old_player).await;

        other_player.receives_offer_to_join_round().await;
        other_player.joins_round().await;
        new_player.receives_offer_to_join_round().await;
        new_player.joins_round().await;

        play_round(&mut other_player, &mut new_player).await;

        new_player
            .receives_scores(&[("theOtherPlayer", 0), ("thePlayer", 2)])
            .await;
    }
}

mod with_two_registered_players {
    use super::*;

    async fn player1_loses_round(players: &mut [FakeClient]) {
        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());
        players[0].is_asked_to_play_a_turn().await;
        players[0].wants_to_see().await;
        each!(
            players,
            receives_notification_that_player_lost("client1", "SEE_BEFORE_FIRST_ROLL")
        );
    }

    #[tokio::test]
    async fn hosts_a_round_with_a_player_calling_and_losing() {
        let server = TestServer::start().await;
        let mut players = server.players(&["client1", "client2"]).await;
        server.server_rolls(dice(6, 6));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());
        each!(
            players,
            receives_notification_that_round_is_starting(1, &["client1", "client2"])
        );

        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        each!(players, receives_notification_that_player_rolls("client1"));
        players[0].receives_rolled_dice(dice(6, 6)).await;
        players[0].announces_dice(dice(6, 6)).await;

        each!(players, receives_dice_announcement("client1", dice(6, 6)));

        players[1].is_asked_to_play_a_turn().await;
        players[1].wants_to_see().await;

        each!(
            players,
            receives_notification_that_player_wants_to_see("client2")
        );
        each!(players, receives_actual_dice(dice(6, 6)));
        each!(
            players,
            receives_notification_that_player_lost("client2", "SEE_FAILED")
        );
        each!(players, receives_scores(&[("client1", 1), ("client2", 0)]));
    }

    #[tokio::test]
    async fn hosts_a_round_with_a_player_calling_and_winning() {
        let server = TestServer::start().await;
        let mut players = server.players(&["client1", "client2"]).await;
        server.server_rolls(dice(4, 4));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());
        each!(
            players,
            receives_notification_that_round_is_starting(1, &["client1", "client2"])
        );

        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        players[0].receives_rolled_dice(dice(4, 4)).await;
        players[0].announces_dice(dice(6, 6)).await;

        each!(players, receives_dice_announcement("client1", dice(6, 6)));

        players[1].is_asked_to_play_a_turn().await;
        players[1].wants_to_see().await;

        each!(players, receives_actual_dice(dice(4, 4)));
        each!(
            players,
            receives_notification_that_player_lost("client1", "CAUGHT_BLUFFING")
        );
        each!(players, receives_scores(&[("client1", 0), ("client2", 1)]));
    }

    #[tokio::test]
    async fn keeps_the_score_across_multiple_rounds() {
        let server = TestServer::start().await;
        let mut players = server.players(&["client1", "client2"]).await;

        player1_loses_round(&mut players).await;
        each!(players, receives_scores(&[("client1", 0), ("client2", 1)]));

        player1_loses_round(&mut players).await;
        each!(players, receives_scores(&[("client1", 0), ("client2", 2)]));
    }
}

mod mia_rules {
    use super::*;

    #[tokio::test]
    async fn when_mia_is_announced_all_other_players_immediately_lose() {
        let server = TestServer::start().await;
        let mut players = server.players(&["client1", "client2", "client3"]).await;
        server.server_rolls(mia());

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());

        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        players[0].receives_rolled_dice(mia()).await;
        players[0].announces_dice(mia()).await;

        each!(players, receives_dice_announcement("client1", mia()));
        each!(players, receives_actual_dice(mia()));
        each!(
            players,
            receives_notification_that_players_lost(&["client2", "client3"], "MIA")
        );
        each!(
            players,
            receives_scores(&[("client1", 1), ("client2", 0), ("client3", 0)])
        );
    }

    #[tokio::test]
    async fn when_mia_is_announced_wrongly_the_player_immediately_loses() {
        let server = TestServer::start().await;
        let mut players = server.players(&["client1", "client2", "client3"]).await;
        server.server_rolls(dice(3, 1));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());

        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        players[0].receives_rolled_dice(dice(3, 1)).await;
        players[0].announces_dice(mia()).await;

        each!(players, receives_dice_announcement("client1", mia()));
        each!(players, receives_actual_dice(dice(3, 1)));
        each!(
            players,
            receives_notification_that_player_lost("client1", "LIED_ABOUT_MIA")
        );
        each!(
            players,
            receives_scores(&[("client1", 0), ("client2", 1), ("client3", 1)])
        );
    }
}

mod half_mia_rules {
    use super::*;

    #[tokio::test]
    async fn the_half_mia_is_a_very_high_dice_that_is_not_revealed() {
        let server = TestServer::start_with(RuleSet::Fixed(Rules::new(
            MiaRule::Mia42,
            BlindRoll::Forbidden,
        )))
        .await;
        let mut players = server.players(&["client1", "client2"]).await;
        server.server_rolls(dice(3, 1));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());

        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        players[0].receives_rolled_dice(dice(3, 1)).await;
        players[0].announces_dice(dice(2, 1)).await;
        each!(players, receives_dice_announcement("client1", dice(2, 1)));

        players[1].is_asked_to_play_a_turn().await;
        players[1].wants_to_see().await;
        each!(players, receives_actual_dice(dice(3, 1)));
        each!(
            players,
            receives_notification_that_player_lost("client1", "CAUGHT_BLUFFING")
        );
    }

    #[tokio::test]
    async fn four_two_is_the_mia_that_makes_all_other_players_lose() {
        let server = TestServer::start_with(RuleSet::Fixed(Rules::new(
            MiaRule::Mia42,
            BlindRoll::Forbidden,
        )))
        .await;
        let mut players = server.players(&["client1", "client2"]).await;
        server.server_rolls(dice(4, 2));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());

        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        players[0].receives_rolled_dice(dice(4, 2)).await;
        players[0].announces_dice(dice(4, 2)).await;

        each!(players, receives_dice_announcement("client1", dice(4, 2)));
        each!(players, receives_actual_dice(dice(4, 2)));
        each!(
            players,
            receives_notification_that_player_lost("client2", "MIA")
        );
    }

    #[tokio::test]
    async fn the_half_mia_beats_the_highest_pair() {
        let server = TestServer::start_with(RuleSet::Fixed(Rules::new(
            MiaRule::Mia42,
            BlindRoll::Forbidden,
        )))
        .await;
        let mut players = server.players(&["client1", "client2"]).await;
        server.server_rolls(dice(3, 1));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());

        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        players[0].receives_rolled_dice(dice(3, 1)).await;
        players[0].announces_dice(dice(6, 6)).await;
        players[1].is_asked_to_play_a_turn().await;
        players[1].rolls().await;
        players[1].receives_rolled_dice(dice(3, 1)).await;
        players[1].announces_dice(dice(2, 1)).await;

        players[0].is_asked_to_play_a_turn().await;
    }
}

mod blind_roll_rules {
    use super::*;

    #[tokio::test]
    async fn a_blind_roll_replaces_the_first_roll_without_showing_the_dice() {
        let server = TestServer::start_with(RuleSet::Fixed(Rules::new(
            MiaRule::Mia21,
            BlindRoll::Allowed,
        )))
        .await;
        let mut players = server.players(&["client1", "client2"]).await;
        server.server_rolls(dice(4, 4));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());

        players[0].is_asked_to_play_a_turn().await;
        players[0].blind_rolls().await;
        each!(
            players,
            receives_notification_that_player_blind_rolls("client1")
        );
        players[0].receives_blind_rolled_confirmation().await;
        players[0].announces_dice(dice(6, 6)).await;
        each!(players, receives_dice_announcement("client1", dice(6, 6)));

        players[1].is_asked_to_play_a_turn().await;
        players[1].wants_to_see().await;
        each!(players, receives_actual_dice(dice(4, 4)));
        each!(
            players,
            receives_notification_that_player_lost("client1", "CAUGHT_BLUFFING")
        );
    }

    #[tokio::test]
    async fn a_blind_roll_after_a_seen_roll_replaces_the_dice_unseen() {
        let server = TestServer::start_with(RuleSet::Fixed(Rules::new(
            MiaRule::Mia21,
            BlindRoll::Allowed,
        )))
        .await;
        let mut players = server.players(&["client1", "client2"]).await;
        server.server_rolls(dice(3, 1));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());

        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        players[0].receives_rolled_dice(dice(3, 1)).await;
        server.server_rolls(dice(5, 5));
        players[0].blind_rolls().await;
        players[0].receives_blind_rolled_confirmation().await;
        players[0].announces_dice(dice(4, 4)).await;

        players[1].is_asked_to_play_a_turn().await;
        players[1].wants_to_see().await;
        each!(players, receives_actual_dice(dice(5, 5)));
        each!(
            players,
            receives_notification_that_player_lost("client2", "SEE_FAILED")
        );
    }
}

mod replaced_player {
    use super::*;

    #[tokio::test]
    async fn the_old_client_can_still_unregister() {
        let server = TestServer::start().await;
        let mut old_player = server.player("thePlayer").await;
        let _new_player = server.player("thePlayer").await;

        old_player.send("UNREGISTER").await;

        old_player.receives("UNREGISTERED").await;
    }
}

mod heartbeat {
    use super::*;

    #[tokio::test]
    async fn registered_clients_receive_heartbeats() {
        let server = TestServer::start().await;
        let mut player = server.player("thePlayer").await;

        player.receives_a_heartbeat().await;
    }
}

mod timeouts {
    use super::*;

    #[tokio::test]
    async fn a_player_missing_the_turn_loses_and_spectators_see_the_score() {
        let server = TestServer::start().await;
        let mut spectator = server.spectator("theSpectator").await;
        let mut players = server.players(&["client1", "client2"]).await;

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());
        players[0].is_asked_to_play_a_turn().await;

        spectator
            .receives_notification_that_player_lost("client1", "DID_NOT_TAKE_TURN")
            .await;
        spectator
            .receives_scores(&[("client1", 0), ("client2", 1)])
            .await;
    }

    #[tokio::test]
    async fn a_player_missing_the_announcement_loses() {
        let server = TestServer::start().await;
        let mut players = server.players(&["client1", "client2"]).await;
        server.server_rolls(dice(3, 1));

        each!(players, receives_offer_to_join_round());
        each!(players, joins_round());
        players[0].is_asked_to_play_a_turn().await;
        players[0].rolls().await;
        players[0].receives_rolled_dice(dice(3, 1)).await;

        each!(
            players,
            receives_notification_that_player_lost("client1", "DID_NOT_ANNOUNCE")
        );
        each!(players, receives_scores(&[("client1", 0), ("client2", 1)]));
    }
}
