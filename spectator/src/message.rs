//! The messages of the server a spectator cares about, as typed values.

use core::{fmt, str::FromStr};

/// Two dice as the server writes them, the higher die first (`6,5`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dice {
    /// The higher die.
    pub high: u8,
    /// The lower die.
    pub low: u8,
}

impl fmt::Display for Dice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.high, self.low)
    }
}

impl FromStr for Dice {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (first, second) = text.split_once(',').ok_or(())?;
        let first: u8 = first.parse().map_err(|_| ())?;
        let second: u8 = second.parse().map_err(|_| ())?;
        Ok(Self {
            high: first.max(second),
            low: first.min(second),
        })
    }
}

/// Why a player lost a round.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LossReason {
    /// Wanted to see before anybody rolled.
    SeeBeforeFirstRoll,
    /// Announced Mia without having it.
    LiedAboutMia,
    /// Announced dice that do not beat the previous announcement.
    AnnouncedLosingDice,
    /// Did not announce in time.
    DidNotAnnounce,
    /// Did not take a turn in time.
    DidNotTakeTurn,
    /// Made an invalid turn.
    InvalidTurn,
    /// Wanted to see, but the previous announcement was true.
    SeeFailed,
    /// Announced more than rolled and was challenged.
    CaughtBluffing,
    /// Mia was rolled and announced.
    Mia,
    /// A reason this spectator does not know.
    Other(String),
}

impl From<&str> for LossReason {
    fn from(reason: &str) -> Self {
        match reason {
            "SEE_BEFORE_FIRST_ROLL" => Self::SeeBeforeFirstRoll,
            "LIED_ABOUT_MIA" => Self::LiedAboutMia,
            "ANNOUNCED_LOSING_DICE" => Self::AnnouncedLosingDice,
            "DID_NOT_ANNOUNCE" => Self::DidNotAnnounce,
            "DID_NOT_TAKE_TURN" => Self::DidNotTakeTurn,
            "INVALID_TURN" => Self::InvalidTurn,
            "SEE_FAILED" => Self::SeeFailed,
            "CAUGHT_BLUFFING" => Self::CaughtBluffing,
            "MIA" => Self::Mia,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Why a round was canceled.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CancelReason {
    /// Nobody wanted to play.
    NoPlayers,
    /// Only one player joined.
    OnlyOnePlayer,
    /// A reason this spectator does not know.
    Other(String),
}

impl From<&str> for CancelReason {
    fn from(reason: &str) -> Self {
        match reason {
            "NO_PLAYERS" => Self::NoPlayers,
            "ONLY_ONE_PLAYER" => Self::OnlyOnePlayer,
            other => Self::Other(other.to_string()),
        }
    }
}

/// The rules a server announces for a round, written `mia21`, `mia42` or `mia42,blind`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnnouncedRules {
    /// 4,2 is the Mia and 2,1 the half Mia.
    pub half_mia: bool,
    /// Blind rolls are allowed.
    pub blind_roll: bool,
}

impl FromStr for AnnouncedRules {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (mia, option) = text
            .split_once(',')
            .map_or((text, None), |(mia, option)| (mia, Some(option)));
        let half_mia = match mia {
            "mia21" => false,
            "mia42" => true,
            _ => return Err(()),
        };
        let blind_roll = match option {
            None => false,
            Some("blind") => true,
            Some(_) => return Err(()),
        };
        Ok(Self {
            half_mia,
            blind_roll,
        })
    }
}

/// A message of the server that changes what a spectator shows.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Message {
    /// A round starts with these players in turn order.
    RoundStarted {
        /// Number of the round.
        number: u64,
        /// The players in turn order.
        players: Vec<String>,
        /// The rules of the round, if the server announces them (random rules).
        rules: Option<AnnouncedRules>,
    },
    /// A round was canceled before the first turn.
    RoundCanceled(CancelReason),
    /// A player rolls and looks at the dice.
    PlayerRolls(String),
    /// A player rolls without looking.
    PlayerBlindRolls(String),
    /// A player wants to see the dice.
    PlayerWantsToSee(String),
    /// A player announced dice.
    Announced {
        /// The announcing player.
        player: String,
        /// The announced dice.
        dice: Dice,
    },
    /// The dice that were actually rolled.
    ActualDice(Dice),
    /// The round ended with these losers.
    PlayerLost {
        /// The players that lost the round.
        players: Vec<String>,
        /// Why they lost.
        reason: LossReason,
    },
    /// The current scores.
    Score(Vec<(String, i32)>),
}

/// Reads a message of the server; `None` for messages that are unknown or of no interest.
#[must_use]
pub fn parse(command: &str, parameters: &[&str]) -> Option<Message> {
    match (command, parameters) {
        ("ROUND STARTED", [number, players]) => Some(Message::RoundStarted {
            number: number.parse().ok()?,
            players: names(players),
            rules: None,
        }),
        ("ROUND STARTED", [number, rules, players]) => Some(Message::RoundStarted {
            number: number.parse().ok()?,
            players: names(players),
            rules: Some(rules.parse().ok()?),
        }),
        ("ROUND CANCELED", [reason]) => Some(Message::RoundCanceled((*reason).into())),
        ("PLAYER ROLLS", [player]) => Some(Message::PlayerRolls((*player).to_string())),
        ("PLAYER BLIND ROLLS", [player]) => Some(Message::PlayerBlindRolls((*player).to_string())),
        ("PLAYER WANTS TO SEE", [player]) => Some(Message::PlayerWantsToSee((*player).to_string())),
        ("ANNOUNCED", [player, dice]) => Some(Message::Announced {
            player: (*player).to_string(),
            dice: dice.parse().ok()?,
        }),
        ("ACTUAL DICE", [dice]) => Some(Message::ActualDice(dice.parse().ok()?)),
        ("PLAYER LOST", [players, reason]) => Some(Message::PlayerLost {
            players: names(players),
            reason: (*reason).into(),
        }),
        ("SCORE", [scores]) => Some(Message::Score(scores_of(scores)?)),
        _ => None,
    }
}

fn names(list: &str) -> Vec<String> {
    list.split(',')
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn scores_of(list: &str) -> Option<Vec<(String, i32)>> {
    list.split(',')
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let (name, points) = entry.split_once(':')?;
            Some((name.to_string(), points.parse().ok()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{AnnouncedRules, CancelReason, Dice, LossReason, Message, parse};

    fn parse_line(line: &str) -> Option<Message> {
        let mut parts = line.split(';');
        let command = parts.next()?;
        let parameters: Vec<&str> = parts.collect();
        parse(command, &parameters)
    }

    #[test]
    fn round_started_lists_players_in_turn_order() {
        assert_eq!(
            parse_line("ROUND STARTED;3;bert,anna"),
            Some(Message::RoundStarted {
                number: 3,
                players: vec!["bert".to_string(), "anna".to_string()],
                rules: None,
            })
        );
    }

    #[test]
    fn announced_rules_are_read_from_the_second_parameter() {
        assert_eq!(
            parse_line("ROUND STARTED;3;mia42,blind;bert,anna"),
            Some(Message::RoundStarted {
                number: 3,
                players: vec!["bert".to_string(), "anna".to_string()],
                rules: Some(AnnouncedRules {
                    half_mia: true,
                    blind_roll: true
                }),
            })
        );
        assert_eq!(
            parse_line("ROUND STARTED;3;mia21;bert,anna"),
            Some(Message::RoundStarted {
                number: 3,
                players: vec!["bert".to_string(), "anna".to_string()],
                rules: Some(AnnouncedRules::default()),
            })
        );
        assert_eq!(parse_line("ROUND STARTED;3;7;bert,anna"), None);
    }

    #[test]
    fn dice_are_normalized_with_the_higher_die_first() {
        assert_eq!(
            parse_line("ACTUAL DICE;1,6"),
            Some(Message::ActualDice(Dice { high: 6, low: 1 }))
        );
    }

    #[test]
    fn score_without_players_is_empty() {
        assert_eq!(parse_line("SCORE;"), Some(Message::Score(vec![])));
    }

    #[test]
    fn score_with_a_broken_entry_is_ignored() {
        assert_eq!(parse_line("SCORE;anna:1,bert:x"), None);
    }

    #[test]
    fn unknown_reasons_are_kept() {
        assert_eq!(
            parse_line("PLAYER LOST;anna;SOMETHING_NEW"),
            Some(Message::PlayerLost {
                players: vec!["anna".to_string()],
                reason: LossReason::Other("SOMETHING_NEW".to_string()),
            })
        );
        assert_eq!(
            parse_line("ROUND CANCELED;SOMETHING_NEW"),
            Some(Message::RoundCanceled(CancelReason::Other(
                "SOMETHING_NEW".to_string()
            )))
        );
    }

    #[test]
    fn dice_are_written_with_a_comma() {
        assert_eq!(Dice { high: 6, low: 1 }.to_string(), "6,1");
    }

    #[test]
    fn announced_rules_know_only_blind_as_an_option() {
        assert!("mia42,fast".parse::<AnnouncedRules>().is_err());
        assert!("mia42,blind".parse::<AnnouncedRules>().is_ok());
    }

    #[test]
    fn unknown_messages_are_ignored() {
        assert_eq!(parse_line("SOMETHING NEW;42"), None);
        assert_eq!(parse_line("ANNOUNCED;anna;nonsense"), None);
    }
}
