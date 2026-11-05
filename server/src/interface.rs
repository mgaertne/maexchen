use core::str::FromStr;

use derive_more::Display;
use log::Level;
use thiserror::Error;

use super::SimpleDice;
use crate::Rules;

/// Why a text is not a valid [`ClientCommand`].
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseCommandError {
    /// The text is no command the server knows.
    #[error("invalid client command")]
    UnknownCommand,
    /// An `ANNOUNCE` lacks valid dice or the token.
    #[error("wrong announcement format")]
    WrongAnnouncement,
}

/// A request a client sends to the server, in the wire format of the protocol.
///
/// ```
/// use maexchen::ClientCommand;
///
/// let command: ClientCommand = "ANNOUNCE;6,5;token".parse().unwrap();
///
/// assert_eq!(command.to_string(), "ANNOUNCE;6,5;token");
/// assert!("DANCE;now".parse::<ClientCommand>().is_err());
/// ```
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClientCommand {
    /// Asks the server to repeat the text (`ECHO;<text>`); several arguments are joined.
    #[display("ECHO;{_0}")]
    Echo(String),
    /// Registers a player under the given name (`REGISTER;<name>`).
    #[display("REGISTER;{_0}")]
    Register(String),
    /// Registers a spectator under the given name (`REGISTER_SPECTATOR;<name>`).
    #[display("REGISTER_SPECTATOR;{_0}")]
    RegisterSpectator(String),
    /// Leaves the game (`UNREGISTER`); arguments after a `;` are ignored.
    #[display("UNREGISTER")]
    Unregister,
    /// Accepts the invitation to a round (`JOIN;<token>`).
    #[display("JOIN;{_0}")]
    Join(String),
    /// Rolls the dice on the own turn (`ROLL;<token>`).
    #[display("ROLL;{_0}")]
    Roll(String),
    /// Rolls the dice without seeing them (`BLIND_ROLL;<token>`);
    #[display("BLIND_ROLL;{_0}")]
    BlindRoll(String),
    /// Wants to see the dice of the previous player (`SEE;<token>`).
    #[display("SEE;{_0}")]
    See(String),
    /// Announces dice after rolling (`ANNOUNCE;<dice>;<token>`).
    #[display("ANNOUNCE;{_0};{_1}")]
    Announce(SimpleDice, String),
}

impl FromStr for ClientCommand {
    type Err = ParseCommandError;

    fn from_str(s: &str) -> Result<Self, ParseCommandError> {
        if s.split(';').next() == Some("UNREGISTER") {
            return Ok(Self::Unregister);
        }
        if s == "ECHO" {
            return Ok(Self::Echo(String::new()));
        }

        match s.split_once(';') {
            Some(("ECHO", parts)) => Ok(Self::Echo(parts.split(';').collect())),
            Some(("REGISTER", client_name)) => Ok(Self::Register(client_name.to_string())),
            Some(("REGISTER_SPECTATOR", spectator_name)) => {
                Ok(Self::RegisterSpectator(spectator_name.to_string()))
            }
            Some(("JOIN", token)) => Ok(Self::Join(token.to_string())),
            Some(("ROLL", token)) => Ok(Self::Roll(token.to_string())),
            Some(("BLIND_ROLL", token)) => Ok(Self::BlindRoll(token.to_string())),
            Some(("SEE", token)) => Ok(Self::See(token.to_string())),
            Some(("ANNOUNCE", parts)) => {
                if let Some((dice_str, token)) = parts.split_once(';')
                    && let Ok(dice) = dice_str.parse()
                {
                    return Ok(Self::Announce(dice, token.to_string()));
                }
                Err(ParseCommandError::WrongAnnouncement)
            }
            _ => Err(ParseCommandError::UnknownCommand),
        }
    }
}

#[cfg(test)]
mod client_command_tests {
    use rstest::rstest;

    use super::{ClientCommand, ParseCommandError, SimpleDice};

    #[rstest]
    #[case(ClientCommand::Echo("Hello World".to_string()), "ECHO;Hello World")]
    #[case(ClientCommand::Register("bot-name".to_string()), "REGISTER;bot-name")]
    #[case(ClientCommand::RegisterSpectator("spec-name".to_string()), "REGISTER_SPECTATOR;spec-name")]
    #[case(ClientCommand::Unregister, "UNREGISTER")]
    #[case(ClientCommand::Join("asdf".to_string()), "JOIN;asdf")]
    #[case(ClientCommand::Roll("asdf".to_string()), "ROLL;asdf")]
    #[case(ClientCommand::BlindRoll("asdf".to_string()), "BLIND_ROLL;asdf")]
    #[case(ClientCommand::See("asdf".to_string()), "SEE;asdf")]
    #[case(ClientCommand::Announce(SimpleDice::new(6, 5), "asdf".to_string()), "ANNOUNCE;6,5;asdf")]
    fn converts_to_and_from_the_interface_string(#[case] cmd: ClientCommand, #[case] text: &str) {
        assert_eq!(cmd.to_string(), text);
        assert_eq!(text.parse::<ClientCommand>().unwrap(), cmd);
    }

    #[test]
    fn bare_echo_is_an_echo_without_text() {
        assert_eq!(
            "ECHO".parse::<ClientCommand>().unwrap(),
            ClientCommand::Echo(String::new())
        );
    }

    #[rstest]
    #[case("UNREGISTER;")]
    #[case("UNREGISTER;ignored")]
    fn unregister_ignores_trailing_arguments(#[case] text: &str) {
        assert_eq!(
            text.parse::<ClientCommand>().unwrap(),
            ClientCommand::Unregister
        );
    }

    #[test]
    fn announced_dice_may_come_in_either_order() {
        assert_eq!(
            "ANNOUNCE;2,4;asdf".parse::<ClientCommand>().unwrap(),
            ClientCommand::Announce(SimpleDice::new(4, 2), "asdf".to_string())
        );
    }

    #[rstest]
    #[case("ANNOUNCE;7,1;asdf")]
    #[case("ANNOUNCE;0,3;asdf")]
    #[case("ANNOUNCE;42;asdf")]
    fn announced_dice_out_of_range_cannot_be_parsed(#[case] text: &str) {
        assert_eq!(
            text.parse::<ClientCommand>(),
            Err(ParseCommandError::WrongAnnouncement)
        );
    }

    #[test]
    fn wrong_announcement_format_cannot_be_parsed() {
        assert_eq!(
            "ANNOUNCE;4;2;asdf".parse::<ClientCommand>(),
            Err(ParseCommandError::WrongAnnouncement)
        );
    }

    #[test]
    fn unknown_client_comand_cannot_be_parsed() {
        assert_eq!(
            "UNKNOWN COMMAND;asdf".parse::<ClientCommand>(),
            Err(ParseCommandError::UnknownCommand)
        );
    }
}

/// Why a registration was refused.
#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum RejectionReason {
    /// The name is empty, too long or contains forbidden characters.
    InvalidName,
    /// Another host already uses the name.
    NameAlreadyTaken,
}

/// Why a round was called off before it began.
#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum CancellationReason {
    /// Nobody joined the round.
    NoPlayers,
    /// Only one player joined the round.
    OnlyOnePlayer,
}

/// Why players lost a round.
#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum LosingReason {
    /// The player wanted to see the dice before anything was announced.
    SeeBeforeFirstRoll,
    /// The player announced the Mia without having rolled it.
    LiedAboutMia,
    /// The player announced dice that do not beat the previous announcement.
    AnnouncedLosingDice,
    /// The player did not announce in time after rolling.
    DidNotAnnounce,
    /// The player did not answer in time on the own turn.
    DidNotTakeTurn,
    #[allow(
        dead_code,
        reason = "the protocol defines it, but the server never detects an invalid turn"
    )]
    /// The player made an invalid turn.
    InvalidTurn,
    /// The player wanted to see the dice, but the announcement was true.
    SeeFailed,
    /// The previous player announced higher dice than rolled.
    CaughtBluffing,
    /// Somebody announced the Mia; everyone else loses.
    Mia,
}

/// A message the server sends to a client, in the wire format of the protocol.
///
/// ```
/// use maexchen::{LosingReason, PlayerMessage};
///
/// let message = PlayerMessage::PlayerLost(vec!["anna".to_string()], LosingReason::SeeFailed);
///
/// assert_eq!(message.to_string(), "PLAYER LOST;anna;SEE_FAILED");
/// ```
#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum PlayerMessage {
    /// Confirms a registration.
    Registered,
    /// Refuses a registration (`REJECTED;<reason>`).
    #[display("REJECTED;{_0}")]
    RegistrationRejected(RejectionReason),
    /// Confirms an unregistration.
    Unregistered,
    /// Shows that the server is alive; sent regularly to every registered client.
    Heartbeat,
    /// Answers an echo request (`ECHOING;<text>`).
    #[display("ECHOING;{_0}")]
    Echoing(String),
    /// Invites a player to the next round (`ROUND STARTING;<token>`).
    #[display("ROUND STARTING;{_0}")]
    RoundStarting(String),
    /// Tells a player that it is the turn (`YOUR TURN;<token>`).
    #[display("YOUR TURN;{_0}")]
    YourTurn(String),
    /// Tells everybody who rolls (`PLAYER ROLLS;<player>`).
    #[display("PLAYER ROLLS;{_0}")]
    PlayerRolls(String),
    /// Tells everybody who rolls blind (`PLAYER BLIND ROLLS;<player>`).
    #[display("PLAYER BLIND ROLLS;{_0}")]
    PlayerBlindRolls(String),
    /// Tells everybody who wants to see the dice (`PLAYER WANTS TO SEE;<player>`).
    #[display("PLAYER WANTS TO SEE;{_0}")]
    PlayerWantsToSee(String),
    /// Shows the rolled dice to the rolling player (`ROLLED;<dice>;<token>`).
    #[display("ROLLED;{_0};{_1}")]
    YourRoll(SimpleDice, String),
    /// Confirms a blind roll without showing the dice (`BLIND_ROLLED;<token>`).
    #[display("BLIND_ROLLED;{_0}")]
    YourBlindRoll(String),
    /// Tells everybody why the round was called off (`ROUND CANCELED;<reason>`).
    #[display("ROUND CANCELED;{_0}")]
    RoundCanceled(CancellationReason),
    /// Tells everybody the number, the rules and the players of the round in turn order.
    ///
    /// The rules are only announced (`ROUND STARTED;<round>;<rules>;<players>`) when they change
    /// from round to round, with random rules.
    #[display("ROUND STARTED;{_0};{}{}", _1.map(|rules| format!("{rules};")).unwrap_or_default(), _2.join(","))]
    RoundStarted(u64, Option<Rules>, Vec<String>),
    /// Tells everybody the dice a player announced (`ANNOUNCED;<player>;<dice>`).
    #[display("ANNOUNCED;{_0};{_1}")]
    AnnouncedDiceBy(String, SimpleDice),
    /// Reveals the dice that were really rolled (`ACTUAL DICE;<dice>`).
    #[display("ACTUAL DICE;{_0}")]
    ActualDice(SimpleDice),
    /// Tells everybody who lost the round and why (`PLAYER LOST;<players>;<reason>`).
    #[display("PLAYER LOST;{};{_1}", _0.join(","))]
    PlayerLost(Vec<String>, LosingReason),
    /// Tells everybody the scores as `<player>:<points>` pairs (`SCORE;<scores>`).
    #[display("SCORE;{}", _0.iter().map(|(n, s)| format!("{n}:{s}")).collect::<Vec<String>>().join(","))]
    CurrentScore(Vec<(String, i32)>),
}

impl PlayerMessage {
    /// Heartbeats are logged at debug level, everything else at info level.
    ///
    /// ```
    /// use log::Level;
    /// use maexchen::PlayerMessage;
    ///
    /// assert_eq!(PlayerMessage::Heartbeat.loglevel(), Level::Debug);
    /// assert_eq!(PlayerMessage::Registered.loglevel(), Level::Info);
    /// ```
    #[must_use]
    pub fn loglevel(&self) -> Level {
        match self {
            Self::Heartbeat => Level::Debug,
            _ => Level::Info,
        }
    }
}

#[cfg(test)]
mod player_message_tests {
    use log::Level;
    use rstest::rstest;

    use super::{CancellationReason, LosingReason, PlayerMessage, RejectionReason, SimpleDice};
    use crate::{BlindRoll, MiaRule, Rules};

    #[rstest]
    #[case(PlayerMessage::Registered, "REGISTERED")]
    #[case(
        PlayerMessage::RegistrationRejected(RejectionReason::InvalidName),
        "REJECTED;INVALID_NAME"
    )]
    #[case(
        PlayerMessage::RegistrationRejected(RejectionReason::NameAlreadyTaken),
        "REJECTED;NAME_ALREADY_TAKEN"
    )]
    #[case(PlayerMessage::Unregistered, "UNREGISTERED")]
    #[case(PlayerMessage::Heartbeat, "HEARTBEAT")]
    #[case(PlayerMessage::Echoing("hi".to_string()), "ECHOING;hi")]
    #[case(PlayerMessage::RoundStarting("42".to_string()), "ROUND STARTING;42")]
    #[case(PlayerMessage::YourTurn("asdf".to_string()), "YOUR TURN;asdf")]
    #[case(PlayerMessage::PlayerRolls("player-name".to_string()), "PLAYER ROLLS;player-name")]
    #[case(PlayerMessage::PlayerBlindRolls("player-name".to_string()), "PLAYER BLIND ROLLS;player-name")]
    #[case(PlayerMessage::PlayerWantsToSee("player-name".to_string()), "PLAYER WANTS TO SEE;player-name")]
    #[case(PlayerMessage::YourRoll(SimpleDice::new(4, 2), "asdf".to_string()), "ROLLED;4,2;asdf")]
    #[case(PlayerMessage::YourBlindRoll("asdf".to_string()), "BLIND_ROLLED;asdf")]
    #[case(
        PlayerMessage::RoundCanceled(CancellationReason::NoPlayers),
        "ROUND CANCELED;NO_PLAYERS"
    )]
    #[case(
        PlayerMessage::RoundCanceled(CancellationReason::OnlyOnePlayer),
        "ROUND CANCELED;ONLY_ONE_PLAYER"
    )]
    #[case(
        PlayerMessage::RoundStarted(42, None, vec!["player1".to_string(),
        "player2".to_string()]), "ROUND STARTED;42;player1,player2"
    )]
    #[case(
        PlayerMessage::RoundStarted(42, Some(Rules::new(MiaRule::Mia42, BlindRoll::Allowed)),
        vec!["player1".to_string(), "player2".to_string()]),
        "ROUND STARTED;42;mia42,blind;player1,player2"
    )]
    #[case(
        PlayerMessage::AnnouncedDiceBy("player42".to_string(), SimpleDice::new(6, 5)),
        "ANNOUNCED;player42;6,5"
    )]
    #[case(PlayerMessage::ActualDice(SimpleDice::new(4, 2)), "ACTUAL DICE;4,2")]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string()], LosingReason::SeeBeforeFirstRoll),
        "PLAYER LOST;player1;SEE_BEFORE_FIRST_ROLL"
    )]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string()], LosingReason::LiedAboutMia),
        "PLAYER LOST;player1;LIED_ABOUT_MIA"
    )]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string()], LosingReason::AnnouncedLosingDice),
        "PLAYER LOST;player1;ANNOUNCED_LOSING_DICE"
    )]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string()], LosingReason::DidNotAnnounce),
        "PLAYER LOST;player1;DID_NOT_ANNOUNCE"
    )]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string()], LosingReason::DidNotTakeTurn),
        "PLAYER LOST;player1;DID_NOT_TAKE_TURN"
    )]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string()], LosingReason::InvalidTurn),
        "PLAYER LOST;player1;INVALID_TURN"
    )]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string()], LosingReason::SeeFailed),
        "PLAYER LOST;player1;SEE_FAILED"
    )]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string()], LosingReason::CaughtBluffing),
        "PLAYER LOST;player1;CAUGHT_BLUFFING"
    )]
    #[case(
        PlayerMessage::PlayerLost(vec!["player1".to_string(), "player4".to_string()], LosingReason::Mia),
        "PLAYER LOST;player1,player4;MIA"
    )]
    #[case(
        PlayerMessage::CurrentScore(vec![("player1".to_string(), 42), ("player2".to_string(), 21)]),
        "SCORE;player1:42,player2:21"
    )]
    fn player_message_converts_to_interface_str(
        #[case] msg: PlayerMessage,
        #[case] expected: &str,
    ) {
        assert_eq!(msg.to_string(), expected);
    }

    #[test]
    fn heartbeats_are_logged_at_debug_level() {
        assert_eq!(PlayerMessage::Heartbeat.loglevel(), Level::Debug);
    }

    #[test]
    fn other_messages_are_logged_at_info_level() {
        assert_eq!(PlayerMessage::Registered.loglevel(), Level::Info);
        assert_eq!(
            PlayerMessage::YourTurn("asdf".to_string()).loglevel(),
            Level::Info
        );
    }
}

#[cfg(test)]
mod properties {
    use proptest::prelude::*;

    use super::{ClientCommand, SimpleDice};

    const WORD: &str = "[a-zA-Z0-9_-]{1,20}";

    proptest! {
        #[test]
        fn commands_lead_back_to_themselves_through_their_text(
            word in WORD,
            die1 in 1u8..=6,
            die2 in 1u8..=6,
        ) {
            let commands = [
                ClientCommand::Register(word.clone()),
                ClientCommand::RegisterSpectator(word.clone()),
                ClientCommand::Join(word.clone()),
                ClientCommand::Roll(word.clone()),
                ClientCommand::See(word.clone()),
                ClientCommand::Announce(SimpleDice::new(die1, die2), word),
                ClientCommand::Unregister,
            ];

            for command in commands {
                prop_assert_eq!(command.to_string().parse::<ClientCommand>(), Ok(command));
            }
        }

        #[test]
        fn arbitrary_text_never_makes_the_parser_panic(text in any::<String>()) {
            let _ = text.parse::<ClientCommand>();
        }
    }
}
