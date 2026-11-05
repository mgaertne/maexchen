use core::{net::SocketAddr, time::Duration};
use std::collections::VecDeque;

use super::{Effect, Event, Game, TimerId};
use crate::{BlindRoll, MiaRule, RuleSet, Rules, SimpleDice, dealer::Dealer, prelude::*};

const ANSWER_TIMEOUT: Duration = Duration::from_millis(250);

const ANNA: u16 = 1;
const BERT: u16 = 2;
const ZOE: u16 = 3;

const MIA_42_BLIND: Rules = Rules {
    mia: MiaRule::Mia42,
    blind_roll: BlindRoll::Allowed,
};

struct ScriptedDealer {
    rolls: VecDeque<SimpleDice>,
    coins: VecDeque<bool>,
    tokens_issued: u32,
}

impl Dealer for ScriptedDealer {
    fn roll(&mut self) -> SimpleDice {
        self.rolls
            .pop_front()
            .expect("the test scripted fewer rolls than the game asked for")
    }

    fn token(&mut self) -> String {
        self.tokens_issued += 1;
        format!("token-{}", self.tokens_issued)
    }

    fn shuffle(&mut self, _players: &mut [String]) {}

    fn coin(&mut self) -> bool {
        self.coins.pop_front().unwrap_or(false)
    }
}

fn addr(client: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 4000 + client))
}

fn dice(a: u8, b: u8) -> SimpleDice {
    SimpleDice::new(a, b)
}

fn mia() -> SimpleDice {
    dice(2, 1)
}

fn new_game(rolls: &[SimpleDice], start_rounds_early: bool) -> Game<ScriptedDealer> {
    new_game_with(rolls, start_rounds_early, RuleSet::default(), &[])
}

fn new_game_with(
    rolls: &[SimpleDice],
    start_rounds_early: bool,
    rule_set: RuleSet,
    coins: &[bool],
) -> Game<ScriptedDealer> {
    let dealer = ScriptedDealer {
        rolls: rolls.iter().copied().collect(),
        coins: coins.iter().copied().collect(),
        tokens_issued: 0,
    };
    Game::new(dealer, ANSWER_TIMEOUT, start_rounds_early).with_rule_set(rule_set)
}

fn send(game: &mut Game<ScriptedDealer>, client: u16, command: ClientCommand) -> Vec<Effect> {
    game.handle(Event::Received {
        from: addr(client),
        command,
    })
}

fn register(game: &mut Game<ScriptedDealer>, client: u16, name: &str) -> Vec<Effect> {
    send(game, client, ClientCommand::Register(name.to_string()))
}

fn join(game: &mut Game<ScriptedDealer>, client: u16, token: &str) -> Vec<Effect> {
    send(game, client, ClientCommand::Join(token.to_string()))
}

fn roll(game: &mut Game<ScriptedDealer>, client: u16, token: &str) -> Vec<Effect> {
    send(game, client, ClientCommand::Roll(token.to_string()))
}

fn see(game: &mut Game<ScriptedDealer>, client: u16, token: &str) -> Vec<Effect> {
    send(game, client, ClientCommand::See(token.to_string()))
}

fn announce(
    game: &mut Game<ScriptedDealer>,
    client: u16,
    announced: SimpleDice,
    token: &str,
) -> Vec<Effect> {
    send(
        game,
        client,
        ClientCommand::Announce(announced, token.to_string()),
    )
}

fn sent(to: u16, message: PlayerMessage) -> Effect {
    Effect::Send {
        to: addr(to),
        message,
    }
}

fn started_timer(effects: &[Effect]) -> Option<(TimerId, Duration)> {
    effects.iter().find_map(|effect| match effect {
        Effect::StartTimer { id, after } => Some((*id, *after)),
        Effect::Send { .. } => None,
    })
}

fn messages_to(effects: &[Effect], client: u16) -> Vec<PlayerMessage> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Send { to, message } if *to == addr(client) => Some(message.clone()),
            _ => None,
        })
        .collect()
}

fn standings(entries: &[(&str, i32)]) -> PlayerMessage {
    PlayerMessage::CurrentScore(
        entries
            .iter()
            .map(|(name, points)| ((*name).to_string(), *points))
            .collect(),
    )
}

/// Spectator zoe, players anna and bert, and a started round in which anna must
/// roll or see with `token-3`. The answer timer of her turn has the id 2.
fn game_awaiting_annas_turn(rolls: &[SimpleDice]) -> Game<ScriptedDealer> {
    game_awaiting_annas_turn_under(rolls, RuleSet::default(), &[])
}

fn game_awaiting_annas_turn_under(
    rolls: &[SimpleDice],
    rule_set: RuleSet,
    coins: &[bool],
) -> Game<ScriptedDealer> {
    let mut game = new_game_with(rolls, true, rule_set, coins);
    send(
        &mut game,
        ZOE,
        ClientCommand::RegisterSpectator("zoe".to_string()),
    );
    register(&mut game, ANNA, "anna");
    register(&mut game, BERT, "bert");
    game.handle(Event::Start);
    join(&mut game, ANNA, "token-1");
    join(&mut game, BERT, "token-2");
    game
}

mod registration {
    use super::*;

    #[test]
    fn accepts_a_valid_name() {
        let mut game = new_game(&[], true);

        assert_eq!(
            register(&mut game, ANNA, "anna"),
            vec![sent(ANNA, PlayerMessage::Registered)]
        );
    }

    #[test]
    fn rejects_invalid_names() {
        let mut game = new_game(&[], true);

        for name in [
            "",
            "with space",
            "semi;colon",
            "co,mma",
            "col:on",
            &"x".repeat(21),
        ] {
            assert_eq!(
                register(&mut game, ANNA, name),
                vec![sent(
                    ANNA,
                    PlayerMessage::RegistrationRejected(RejectionReason::InvalidName)
                )],
                "name {name:?}"
            );
        }
    }

    #[test]
    fn rejects_a_name_used_by_a_client_on_another_host() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");
        let stranger = SocketAddr::from(([127, 0, 0, 2], 4000));

        let effects = game.handle(Event::Received {
            from: stranger,
            command: ClientCommand::Register("anna".to_string()),
        });

        assert_eq!(
            effects,
            vec![Effect::Send {
                to: stranger,
                message: PlayerMessage::RegistrationRejected(RejectionReason::NameAlreadyTaken)
            }]
        );
    }

    #[test]
    fn lets_a_client_on_the_same_host_take_over_a_name() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");

        assert_eq!(
            register(&mut game, BERT, "anna"),
            vec![sent(BERT, PlayerMessage::Registered)]
        );
        assert_eq!(roll(&mut game, ANNA, "token-1"), vec![]);
    }

    #[test]
    fn allows_the_same_client_to_register_again() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");

        assert_eq!(
            register(&mut game, ANNA, "anna"),
            vec![sent(ANNA, PlayerMessage::Registered)]
        );
    }

    #[test]
    fn tells_spectators_the_scores_when_a_player_registers() {
        let mut game = new_game(&[], true);
        send(
            &mut game,
            ZOE,
            ClientCommand::RegisterSpectator("zoe".to_string()),
        );

        let effects = register(&mut game, ANNA, "anna");

        assert_eq!(messages_to(&effects, ZOE), vec![standings(&[("anna", 0)])]);
    }

    #[test]
    fn unregisters_a_registered_client() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");

        assert_eq!(
            send(&mut game, ANNA, ClientCommand::Unregister),
            vec![sent(ANNA, PlayerMessage::Unregistered)]
        );
        assert_eq!(
            register(&mut game, BERT, "anna"),
            vec![sent(BERT, PlayerMessage::Registered)]
        );
    }

    #[test]
    fn ignores_unregistering_strangers() {
        let mut game = new_game(&[], true);

        assert_eq!(send(&mut game, ANNA, ClientCommand::Unregister), vec![]);
    }
}

mod echo_and_heartbeat {
    use super::*;

    #[test]
    fn echoes_text_without_registration() {
        let mut game = new_game(&[], true);

        assert_eq!(
            send(&mut game, ANNA, ClientCommand::Echo("hi".to_string())),
            vec![sent(ANNA, PlayerMessage::Echoing("hi".to_string()))]
        );
    }

    #[test]
    fn echoes_a_default_for_a_bare_echo() {
        let mut game = new_game(&[], true);

        assert_eq!(
            send(&mut game, ANNA, ClientCommand::Echo(String::new())),
            vec![sent(ANNA, PlayerMessage::Echoing("42".to_string()))]
        );
    }

    #[test]
    fn heartbeat_reaches_every_registered_client() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");
        send(
            &mut game,
            ZOE,
            ClientCommand::RegisterSpectator("zoe".to_string()),
        );

        assert_eq!(
            game.handle(Event::Heartbeat),
            vec![
                sent(ANNA, PlayerMessage::Heartbeat),
                sent(ZOE, PlayerMessage::Heartbeat)
            ]
        );
    }
}

mod starting_rounds {
    use super::*;

    #[test]
    fn waits_for_two_players_before_inviting() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");

        let effects = game.handle(Event::Start);

        assert_eq!(
            effects,
            vec![Effect::StartTimer {
                id: 1,
                after: Duration::from_millis(500)
            }]
        );
    }

    #[test]
    fn invites_every_player_once_two_have_registered() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");
        game.handle(Event::Start);
        register(&mut game, BERT, "bert");

        let effects = game.handle(Event::TimerFired(1));

        assert_eq!(
            messages_to(&effects, ANNA),
            vec![PlayerMessage::RoundStarting("token-1".to_string())]
        );
        assert_eq!(
            messages_to(&effects, BERT),
            vec![PlayerMessage::RoundStarting("token-2".to_string())]
        );
        assert_eq!(started_timer(&effects), Some((2, ANSWER_TIMEOUT)));
    }

    #[test]
    fn does_not_invite_spectators() {
        let mut game = new_game(&[], true);
        send(
            &mut game,
            ZOE,
            ClientCommand::RegisterSpectator("zoe".to_string()),
        );
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "bert");

        let effects = game.handle(Event::Start);

        assert_eq!(messages_to(&effects, ZOE), vec![]);
    }

    #[test]
    fn starts_early_when_everybody_joined() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "bert");
        game.handle(Event::Start);
        join(&mut game, ANNA, "token-1");
        let started =
            PlayerMessage::RoundStarted(1, None, vec!["anna".to_string(), "bert".to_string()]);

        let effects = join(&mut game, BERT, "token-2");

        assert_eq!(
            messages_to(&effects, ANNA),
            vec![
                started.clone(),
                PlayerMessage::YourTurn("token-3".to_string())
            ]
        );
        assert_eq!(messages_to(&effects, BERT), vec![started]);
        assert_eq!(started_timer(&effects), Some((2, ANSWER_TIMEOUT)));
    }

    #[test]
    fn waits_for_the_timeout_when_not_starting_early() {
        let mut game = new_game(&[], false);
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "bert");
        game.handle(Event::Start);
        join(&mut game, ANNA, "token-1");

        assert_eq!(join(&mut game, BERT, "token-2"), vec![]);

        let effects = game.handle(Event::TimerFired(1));
        assert!(effects.contains(&sent(ANNA, PlayerMessage::YourTurn("token-3".to_string()))));
    }

    #[test]
    fn ignores_joins_with_a_wrong_token() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "bert");
        game.handle(Event::Start);
        join(&mut game, ANNA, "token-1");

        assert_eq!(join(&mut game, BERT, "token-1"), vec![]);
    }

    #[test]
    fn cancels_the_round_when_nobody_joins() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "bert");
        game.handle(Event::Start);

        let effects = game.handle(Event::TimerFired(1));

        for client in [ANNA, BERT] {
            let messages = messages_to(&effects, client);
            assert_eq!(
                messages[0],
                PlayerMessage::RoundCanceled(CancellationReason::NoPlayers)
            );
            assert_eq!(messages[1], standings(&[("anna", 0), ("bert", 0)]));
            assert!(matches!(messages[2], PlayerMessage::RoundStarting(_)));
        }
    }

    #[test]
    fn cancels_the_round_when_only_one_player_joins() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "bert");
        game.handle(Event::Start);
        join(&mut game, ANNA, "token-1");

        let effects = game.handle(Event::TimerFired(1));

        assert!(effects.contains(&sent(
            ANNA,
            PlayerMessage::RoundCanceled(CancellationReason::OnlyOnePlayer)
        )));
    }
}

mod turns {
    use super::*;

    #[test]
    fn rolling_hands_the_dice_to_the_roller_only() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);

        let effects = roll(&mut game, ANNA, "token-3");

        assert_eq!(
            messages_to(&effects, ANNA),
            vec![
                PlayerMessage::PlayerRolls("anna".to_string()),
                PlayerMessage::YourRoll(dice(3, 1), "token-4".to_string())
            ]
        );
        assert_eq!(
            messages_to(&effects, BERT),
            vec![PlayerMessage::PlayerRolls("anna".to_string())]
        );
        assert_eq!(
            messages_to(&effects, ZOE),
            vec![PlayerMessage::PlayerRolls("anna".to_string())]
        );
        assert_eq!(started_timer(&effects), Some((3, ANSWER_TIMEOUT)));
    }

    #[test]
    fn announcing_passes_the_turn_on() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        roll(&mut game, ANNA, "token-3");

        let effects = announce(&mut game, ANNA, dice(4, 1), "token-4");

        assert_eq!(
            messages_to(&effects, BERT),
            vec![
                PlayerMessage::AnnouncedDiceBy("anna".to_string(), dice(4, 1)),
                PlayerMessage::YourTurn("token-5".to_string())
            ]
        );
        assert_eq!(started_timer(&effects), Some((4, ANSWER_TIMEOUT)));
    }

    #[test]
    fn ignores_commands_of_players_who_are_not_on_turn() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);

        assert_eq!(roll(&mut game, BERT, "token-3"), vec![]);
        assert_eq!(roll(&mut game, ZOE, "token-3"), vec![]);
    }

    #[test]
    fn ignores_commands_of_other_players_while_an_announcement_is_due() {
        let rules = Rules::new(MiaRule::Mia21, BlindRoll::Allowed);
        let mut game = game_awaiting_annas_turn_under(&[dice(3, 1)], RuleSet::Fixed(rules), &[]);
        roll(&mut game, ANNA, "token-3");

        assert_eq!(announce(&mut game, BERT, dice(4, 1), "token-4"), vec![]);
        assert_eq!(
            send(
                &mut game,
                BERT,
                ClientCommand::BlindRoll("token-4".to_string())
            ),
            vec![]
        );
    }

    #[test]
    fn a_player_who_unregistered_cannot_play_on_in_either_phase() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        send(&mut game, ANNA, ClientCommand::Unregister);
        assert_eq!(roll(&mut game, ANNA, "token-3"), vec![]);

        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        roll(&mut game, ANNA, "token-3");
        send(&mut game, ANNA, ClientCommand::Unregister);
        assert_eq!(announce(&mut game, ANNA, dice(4, 1), "token-4"), vec![]);
    }

    #[test]
    fn ignores_commands_with_a_wrong_token() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);

        assert_eq!(roll(&mut game, ANNA, "token-99"), vec![]);
        assert_ne!(roll(&mut game, ANNA, "token-3"), vec![]);
    }

    #[test]
    fn ignores_an_announcement_before_rolling() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);

        assert_eq!(announce(&mut game, ANNA, dice(4, 1), "token-3"), vec![]);
    }

    #[test]
    fn blind_rolling_is_ignored_when_the_rules_forbid_it() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);

        let effects = send(
            &mut game,
            ANNA,
            ClientCommand::BlindRoll("token-3".to_string()),
        );

        assert_eq!(effects, vec![]);
    }

    #[test]
    fn blind_rolling_hides_the_dice() {
        let rules = Rules::new(MiaRule::Mia21, BlindRoll::Allowed);
        let mut game = game_awaiting_annas_turn_under(&[dice(3, 1)], RuleSet::Fixed(rules), &[]);

        let effects = send(
            &mut game,
            ANNA,
            ClientCommand::BlindRoll("token-3".to_string()),
        );

        assert_eq!(
            messages_to(&effects, ANNA),
            vec![
                PlayerMessage::PlayerBlindRolls("anna".to_string()),
                PlayerMessage::YourBlindRoll("token-4".to_string())
            ]
        );
    }
}

mod losing {
    use super::*;

    fn lost(names: &[&str], reason: LosingReason) -> PlayerMessage {
        PlayerMessage::PlayerLost(
            names.iter().map(|name| (*name).to_string()).collect(),
            reason,
        )
    }

    #[test]
    fn missing_the_turn_loses() {
        let mut game = game_awaiting_annas_turn(&[]);

        let effects = game.handle(Event::TimerFired(2));

        assert_eq!(
            messages_to(&effects, ZOE)[..2],
            [
                lost(&["anna"], LosingReason::DidNotTakeTurn),
                standings(&[("anna", 0), ("bert", 1)])
            ]
        );
    }

    #[test]
    fn missing_the_announcement_loses() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        roll(&mut game, ANNA, "token-3");

        let effects = game.handle(Event::TimerFired(3));

        assert_eq!(
            messages_to(&effects, BERT)[..2],
            [
                lost(&["anna"], LosingReason::DidNotAnnounce),
                standings(&[("anna", 0), ("bert", 1)])
            ]
        );
    }

    #[test]
    fn a_stale_timer_does_nothing() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        roll(&mut game, ANNA, "token-3");

        assert_eq!(game.handle(Event::TimerFired(2)), vec![]);
    }

    #[test]
    fn seeing_before_any_announcement_loses() {
        let mut game = game_awaiting_annas_turn(&[]);

        let effects = see(&mut game, ANNA, "token-3");

        assert_eq!(
            messages_to(&effects, BERT)[..3],
            [
                PlayerMessage::PlayerWantsToSee("anna".to_string()),
                lost(&["anna"], LosingReason::SeeBeforeFirstRoll),
                standings(&[("anna", 0), ("bert", 1)])
            ]
        );
    }

    #[test]
    fn announcing_a_lower_dice_than_before_loses() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1), dice(5, 4)]);
        roll(&mut game, ANNA, "token-3");
        announce(&mut game, ANNA, dice(6, 5), "token-4");
        roll(&mut game, BERT, "token-5");

        let effects = announce(&mut game, BERT, dice(5, 4), "token-6");

        assert_eq!(
            messages_to(&effects, ANNA)[..3],
            [
                PlayerMessage::AnnouncedDiceBy("bert".to_string(), dice(5, 4)),
                lost(&["bert"], LosingReason::AnnouncedLosingDice),
                standings(&[("anna", 1), ("bert", 0)])
            ]
        );
    }

    #[test]
    fn seeing_a_bluff_makes_the_previous_player_lose() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        roll(&mut game, ANNA, "token-3");
        announce(&mut game, ANNA, dice(6, 5), "token-4");

        let effects = see(&mut game, BERT, "token-5");

        assert_eq!(
            messages_to(&effects, ZOE)[..4],
            [
                PlayerMessage::PlayerWantsToSee("bert".to_string()),
                PlayerMessage::ActualDice(dice(3, 1)),
                lost(&["anna"], LosingReason::CaughtBluffing),
                standings(&[("anna", 0), ("bert", 1)])
            ]
        );
    }

    #[test]
    fn seeing_an_honest_announcement_makes_the_seer_lose() {
        let mut game = game_awaiting_annas_turn(&[dice(6, 5)]);
        roll(&mut game, ANNA, "token-3");
        announce(&mut game, ANNA, dice(5, 4), "token-4");

        let effects = see(&mut game, BERT, "token-5");

        assert_eq!(
            messages_to(&effects, ZOE)[..4],
            [
                PlayerMessage::PlayerWantsToSee("bert".to_string()),
                PlayerMessage::ActualDice(dice(6, 5)),
                lost(&["bert"], LosingReason::SeeFailed),
                standings(&[("anna", 1), ("bert", 0)])
            ]
        );
    }

    #[test]
    fn lying_about_mia_loses() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        roll(&mut game, ANNA, "token-3");

        let effects = announce(&mut game, ANNA, mia(), "token-4");

        assert_eq!(
            messages_to(&effects, ZOE)[..4],
            [
                PlayerMessage::AnnouncedDiceBy("anna".to_string(), mia()),
                PlayerMessage::ActualDice(dice(3, 1)),
                lost(&["anna"], LosingReason::LiedAboutMia),
                standings(&[("anna", 0), ("bert", 1)])
            ]
        );
    }

    #[test]
    fn announcing_a_real_mia_makes_everybody_else_lose() {
        let mut game = game_awaiting_annas_turn(&[mia()]);
        roll(&mut game, ANNA, "token-3");

        let effects = announce(&mut game, ANNA, mia(), "token-4");

        assert_eq!(
            messages_to(&effects, ZOE)[..4],
            [
                PlayerMessage::AnnouncedDiceBy("anna".to_string(), mia()),
                PlayerMessage::ActualDice(mia()),
                lost(&["bert"], LosingReason::Mia),
                standings(&[("anna", 1), ("bert", 0)])
            ]
        );
    }

    #[test]
    fn a_new_round_follows_every_decided_round() {
        let mut game = game_awaiting_annas_turn(&[]);

        let effects = game.handle(Event::TimerFired(2));

        assert_eq!(started_timer(&effects), Some((3, ANSWER_TIMEOUT)));
        assert!(matches!(
            messages_to(&effects, ANNA).last(),
            Some(PlayerMessage::RoundStarting(_))
        ));
    }
}

mod half_mia {
    use super::*;

    fn game_awaiting_annas_turn(rolls: &[SimpleDice]) -> Game<ScriptedDealer> {
        let rules = Rules::new(MiaRule::Mia42, BlindRoll::Forbidden);
        game_awaiting_annas_turn_under(rolls, RuleSet::Fixed(rules), &[])
    }

    #[test]
    fn announcing_the_half_mia_is_not_revealed() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        roll(&mut game, ANNA, "token-3");

        let effects = announce(&mut game, ANNA, dice(2, 1), "token-4");

        assert_eq!(
            messages_to(&effects, BERT),
            vec![
                PlayerMessage::AnnouncedDiceBy("anna".to_string(), dice(2, 1)),
                PlayerMessage::YourTurn("token-5".to_string())
            ]
        );
    }
}

mod blind_rolling {
    use super::*;

    fn game_awaiting_annas_turn(rolls: &[SimpleDice]) -> Game<ScriptedDealer> {
        let rules = Rules::new(MiaRule::Mia21, BlindRoll::Allowed);
        game_awaiting_annas_turn_under(rolls, RuleSet::Fixed(rules), &[])
    }

    fn blind_roll(game: &mut Game<ScriptedDealer>, client: u16, token: &str) -> Vec<Effect> {
        send(game, client, ClientCommand::BlindRoll(token.to_string()))
    }

    #[test]
    fn a_blind_roll_can_replace_the_first_roll_and_still_needs_an_announcement() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);
        blind_roll(&mut game, ANNA, "token-3");

        let effects = announce(&mut game, ANNA, dice(4, 1), "token-4");

        assert!(effects.contains(&sent(BERT, PlayerMessage::YourTurn("token-5".to_string()))));
    }

    #[test]
    fn a_blind_roll_after_a_seen_roll_replaces_the_dice_unseen() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1), dice(6, 5)]);
        roll(&mut game, ANNA, "token-3");

        let effects = blind_roll(&mut game, ANNA, "token-4");

        assert_eq!(
            messages_to(&effects, ANNA),
            vec![
                PlayerMessage::PlayerBlindRolls("anna".to_string()),
                PlayerMessage::YourBlindRoll("token-5".to_string())
            ]
        );
        assert_eq!(started_timer(&effects), Some((4, ANSWER_TIMEOUT)));
        announce(&mut game, ANNA, dice(5, 4), "token-5");
        let seen = see(&mut game, BERT, "token-6");
        assert!(seen.contains(&sent(ZOE, PlayerMessage::ActualDice(dice(6, 5)))));
    }

    #[test]
    fn the_old_announcement_token_is_void_after_a_blind_roll() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1), dice(6, 5)]);
        roll(&mut game, ANNA, "token-3");
        blind_roll(&mut game, ANNA, "token-4");

        assert_eq!(announce(&mut game, ANNA, dice(4, 1), "token-4"), vec![]);
    }

    #[test]
    fn a_third_roll_is_not_allowed_after_a_seen_and_a_blind_roll() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1), dice(6, 5), dice(5, 4)]);
        roll(&mut game, ANNA, "token-3");
        blind_roll(&mut game, ANNA, "token-4");

        assert_eq!(blind_roll(&mut game, ANNA, "token-5"), vec![]);
    }

    #[test]
    fn a_blind_first_roll_can_be_rolled_blind_once_more() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1), dice(6, 5), dice(5, 4)]);
        blind_roll(&mut game, ANNA, "token-3");

        let effects = blind_roll(&mut game, ANNA, "token-4");

        assert_eq!(
            messages_to(&effects, ANNA),
            vec![
                PlayerMessage::PlayerBlindRolls("anna".to_string()),
                PlayerMessage::YourBlindRoll("token-5".to_string())
            ]
        );
        assert_eq!(blind_roll(&mut game, ANNA, "token-5"), vec![]);
    }

    #[test]
    fn the_second_roll_cannot_be_a_seen_roll() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1), dice(6, 5)]);
        roll(&mut game, ANNA, "token-3");

        assert_eq!(roll(&mut game, ANNA, "token-4"), vec![]);
    }
}

mod edge_cases {
    use super::*;

    const CLEO: u16 = 4;

    fn lost(names: &[&str], reason: LosingReason) -> PlayerMessage {
        PlayerMessage::PlayerLost(
            names.iter().map(|name| (*name).to_string()).collect(),
            reason,
        )
    }

    /// Players anna, bert and cleo in that order; anna must act on `token-4`.
    fn three_players_awaiting_annas_turn(rolls: &[SimpleDice]) -> Game<ScriptedDealer> {
        let mut game = new_game(rolls, true);
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "bert");
        register(&mut game, CLEO, "cleo");
        game.handle(Event::Start);
        join(&mut game, ANNA, "token-1");
        join(&mut game, BERT, "token-2");
        join(&mut game, CLEO, "token-3");
        game
    }

    #[test]
    fn accepts_a_name_of_the_maximum_length() {
        let mut game = new_game(&[], true);

        assert_eq!(
            register(&mut game, ANNA, &"x".repeat(20)),
            vec![sent(ANNA, PlayerMessage::Registered)]
        );
    }

    #[test]
    fn the_turn_passes_through_all_players_and_wraps_around() {
        let mut game = three_players_awaiting_annas_turn(&[dice(3, 1), dice(3, 1), dice(3, 1)]);
        roll(&mut game, ANNA, "token-4");
        let after_anna = announce(&mut game, ANNA, dice(4, 1), "token-5");
        roll(&mut game, BERT, "token-6");
        let after_bert = announce(&mut game, BERT, dice(5, 1), "token-7");
        roll(&mut game, CLEO, "token-8");

        let after_cleo = announce(&mut game, CLEO, dice(6, 1), "token-9");

        assert!(after_anna.contains(&sent(BERT, PlayerMessage::YourTurn("token-6".to_string()))));
        assert!(after_bert.contains(&sent(CLEO, PlayerMessage::YourTurn("token-8".to_string()))));
        assert!(after_cleo.contains(&sent(ANNA, PlayerMessage::YourTurn("token-10".to_string()))));
    }

    #[test]
    fn a_real_mia_makes_all_other_players_lose() {
        let mut game = three_players_awaiting_annas_turn(&[mia()]);
        roll(&mut game, ANNA, "token-4");

        let effects = announce(&mut game, ANNA, mia(), "token-5");

        assert_eq!(
            messages_to(&effects, ANNA)[2..4],
            [
                lost(&["bert", "cleo"], LosingReason::Mia),
                standings(&[("anna", 1), ("bert", 0), ("cleo", 0)])
            ]
        );
    }

    #[test]
    fn seeing_blames_the_previous_player_even_across_the_wrap() {
        let mut game = three_players_awaiting_annas_turn(&[dice(3, 1), dice(3, 2), dice(3, 2)]);
        roll(&mut game, ANNA, "token-4");
        announce(&mut game, ANNA, dice(4, 1), "token-5");
        roll(&mut game, BERT, "token-6");
        announce(&mut game, BERT, dice(5, 1), "token-7");
        roll(&mut game, CLEO, "token-8");
        announce(&mut game, CLEO, dice(6, 1), "token-9");

        let effects = see(&mut game, ANNA, "token-10");

        assert_eq!(
            messages_to(&effects, BERT)[..4],
            [
                PlayerMessage::PlayerWantsToSee("anna".to_string()),
                PlayerMessage::ActualDice(dice(3, 2)),
                lost(&["cleo"], LosingReason::CaughtBluffing),
                standings(&[("anna", 1), ("bert", 1), ("cleo", 0)])
            ]
        );
    }

    #[test]
    fn announcing_the_same_dice_again_loses() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1), dice(3, 2)]);
        roll(&mut game, ANNA, "token-3");
        announce(&mut game, ANNA, dice(4, 1), "token-4");
        roll(&mut game, BERT, "token-5");

        let effects = announce(&mut game, BERT, dice(4, 1), "token-6");

        assert_eq!(
            messages_to(&effects, ANNA)[..3],
            [
                PlayerMessage::AnnouncedDiceBy("bert".to_string(), dice(4, 1)),
                lost(&["bert"], LosingReason::AnnouncedLosingDice),
                standings(&[("anna", 1), ("bert", 0)])
            ]
        );
    }

    #[test]
    fn a_player_who_unregisters_mid_round_loses_by_timeout_without_hearing_of_it() {
        let mut game = game_awaiting_annas_turn(&[]);
        send(&mut game, ANNA, ClientCommand::Unregister);

        let effects = game.handle(Event::TimerFired(2));

        assert_eq!(messages_to(&effects, ANNA), vec![]);
        assert_eq!(
            messages_to(&effects, BERT)[..2],
            [
                lost(&["anna"], LosingReason::DidNotTakeTurn),
                standings(&[("anna", 0), ("bert", 1)])
            ]
        );
    }

    #[test]
    fn a_player_who_registers_again_mid_round_keeps_the_turn() {
        let mut game = game_awaiting_annas_turn(&[dice(3, 1)]);

        assert_eq!(
            register(&mut game, ANNA, "anna"),
            vec![
                sent(ZOE, standings(&[("anna", 0), ("bert", 0)])),
                sent(ANNA, PlayerMessage::Registered)
            ]
        );
        assert_ne!(roll(&mut game, ANNA, "token-3"), vec![]);
    }

    #[test]
    fn a_join_after_the_round_started_is_ignored() {
        let mut game = game_awaiting_annas_turn(&[]);

        assert_eq!(join(&mut game, BERT, "token-2"), vec![]);
    }
}

mod replaced_clients {
    use super::*;

    #[test]
    fn a_replaced_client_gets_the_confirmation_of_its_unregistration() {
        let mut game = new_game(&[], true);
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "anna");

        assert_eq!(
            send(&mut game, ANNA, ClientCommand::Unregister),
            vec![sent(ANNA, PlayerMessage::Unregistered)]
        );
        assert_eq!(send(&mut game, ANNA, ClientCommand::Unregister), vec![]);
        assert_eq!(
            send(&mut game, BERT, ClientCommand::Unregister),
            vec![sent(BERT, PlayerMessage::Unregistered)]
        );
    }
}

mod random_rules {
    use super::*;

    fn started_with(rule_set: RuleSet, coins: &[bool]) -> Vec<PlayerMessage> {
        let mut game = new_game_with(&[], true, rule_set, coins);
        register(&mut game, ANNA, "anna");
        register(&mut game, BERT, "bert");
        game.handle(Event::Start);
        join(&mut game, ANNA, "token-1");
        let effects = join(&mut game, BERT, "token-2");
        messages_to(&effects, BERT)
    }

    fn started(rules: Option<Rules>) -> PlayerMessage {
        PlayerMessage::RoundStarted(1, rules, vec!["anna".to_string(), "bert".to_string()])
    }

    #[test]
    fn random_rules_are_announced_with_the_round() {
        let messages = started_with(
            RuleSet::Random {
                blind_roll: BlindRoll::Allowed,
            },
            &[true, true],
        );

        assert_eq!(messages, vec![started(Some(MIA_42_BLIND))]);
    }

    #[test]
    fn random_rules_without_blind_rolls_never_allow_them() {
        let messages = started_with(
            RuleSet::Random {
                blind_roll: BlindRoll::Forbidden,
            },
            &[false, true],
        );

        assert_eq!(
            messages,
            vec![started(Some(Rules::new(
                MiaRule::Mia21,
                BlindRoll::Forbidden
            )))]
        );
    }

    #[test]
    fn fixed_rules_are_not_announced() {
        let messages = started_with(RuleSet::Fixed(MIA_42_BLIND), &[]);

        assert_eq!(messages, vec![started(None)]);
    }
}
