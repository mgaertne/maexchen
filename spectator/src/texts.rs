//! German wording for what the spectator shows.

use crate::message::{CancelReason, Dice, LossReason, Message};

/// The rules the server runs with; they decide which dice are called Mäxchen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rules {
    /// The server plays with `mia42`: Mia is 4,2 and 2,1 is only half a Mia.
    pub half_mia: bool,
}

fn group_digits(digits: &str) -> String {
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    grouped
}

/// Writes a whole number with a dot between the thousands: `1.234.567`.
#[must_use]
pub fn grouped(value: impl Into<i128>) -> String {
    let value = value.into();
    let sign = if value < 0 { "-" } else { "" };
    format!("{sign}{}", group_digits(&value.unsigned_abs().to_string()))
}

/// Writes a number the German way: dots between the thousands, a comma before at most two
/// decimals, none if they are zero (`1.234,5`).
#[must_use]
pub fn decimal(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let fixed = format!("{:.2}", value.abs());
    let (whole, fraction) = fixed.split_once('.').unwrap_or((&fixed, ""));
    let fraction = fraction.trim_end_matches('0');
    let sign = if value < 0.0 && fixed != "0.00" {
        "-"
    } else {
        ""
    };
    let comma = if fraction.is_empty() { "" } else { "," };
    format!("{sign}{}{comma}{fraction}", group_digits(whole))
}

/// Names the dice: Mäxchen, a Pasch or the two digits.
#[must_use]
pub fn dice(dice: Dice, rules: Rules) -> String {
    let (mia, half_mia) = if rules.half_mia {
        (Dice { high: 4, low: 2 }, Some(Dice { high: 2, low: 1 }))
    } else {
        (Dice { high: 2, low: 1 }, None)
    };
    if dice == mia {
        "Mäxchen!".to_string()
    } else if Some(dice) == half_mia {
        "Halbes Mäxchen!".to_string()
    } else if dice.high == dice.low {
        let name = match dice.high {
            1 => "Einser",
            2 => "Zweier",
            3 => "Dreier",
            4 => "Vierer",
            5 => "Fünfer",
            _ => "Sechser",
        };
        format!("{name}pasch")
    } else {
        format!("{}{}", dice.high, dice.low)
    }
}

/// Joins names the German way: `anna, bert und carla`.
#[must_use]
pub fn join(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [only] => only.clone(),
        [init @ .., last] => format!("{} und {last}", init.join(", ")),
    }
}

/// Explains why a round was lost.
#[must_use]
pub fn loss_reason(reason: &LossReason) -> String {
    match reason {
        LossReason::SeeBeforeFirstRoll => "wollte als erster am Zug sehen".to_string(),
        LossReason::LiedAboutMia => "wer Mäxchen sagt, sollte Mäxchen haben".to_string(),
        LossReason::AnnouncedLosingDice => {
            "man muss schon mehr sagen als vorher angesagt war".to_string()
        }
        LossReason::DidNotAnnounce => "hat nichts angesagt".to_string(),
        LossReason::DidNotTakeTurn => "hat keinen Spielzug gemacht".to_string(),
        LossReason::InvalidTurn => "wollte einen ungültigen Spielzug machen".to_string(),
        LossReason::SeeFailed => "wollte sehen... aber doch nicht sowas!".to_string(),
        LossReason::CaughtBluffing => "hatte gehofft, dass das keiner überprüft".to_string(),
        LossReason::Mia => "Mäxchen".to_string(),
        LossReason::Other(code) => code.clone(),
    }
}

/// Explains why a round was canceled.
#[must_use]
pub fn cancel_reason(reason: &CancelReason) -> String {
    match reason {
        CancelReason::NoPlayers => "niemand wollte mitspielen".to_string(),
        CancelReason::OnlyOnePlayer => "alleine kann man nicht Mäxchen spielen".to_string(),
        CancelReason::Other(code) => code.clone(),
    }
}

/// Tells in one line what a message means; `None` for the scores, which have their own table.
#[must_use]
pub fn describe(message: &Message, rules: Rules) -> Option<String> {
    match message {
        Message::RoundStarted { players, .. } => Some(format!("Teilnehmer: {}", join(players))),
        Message::RoundCanceled(reason) => {
            Some(format!("Runde abgebrochen: {}", cancel_reason(reason)))
        }
        Message::PlayerRolls(player) => Some(format!("{player} würfelt...")),
        Message::PlayerBlindRolls(player) => Some(format!("{player} würfelt ohne zu gucken...")),
        Message::PlayerWantsToSee(player) => Some(format!("{player} will sehen!")),
        Message::Announced {
            player,
            dice: announced,
        } => Some(format!("{player} sagt an: {}", dice(*announced, rules))),
        Message::ActualDice(actual) => Some(format!(
            "Die Würfel werden aufgedeckt: {}",
            dice(*actual, rules)
        )),
        Message::PlayerLost { players, reason } => {
            let verb = if players.len() == 1 {
                "verliert"
            } else {
                "verlieren"
            };
            Some(format!("{} {verb}: {}", join(players), loss_reason(reason)))
        }
        Message::Score(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Rules, cancel_reason, decimal, describe, dice, grouped, join, loss_reason};
    use crate::message::CancelReason;

    #[test]
    fn whole_numbers_get_a_dot_between_the_thousands() {
        assert_eq!(grouped(0_u8), "0");
        assert_eq!(grouped(999_u16), "999");
        assert_eq!(grouped(1000_u32), "1.000");
        assert_eq!(grouped(26_588_i32), "26.588");
        assert_eq!(grouped(-1_234_567_i64), "-1.234.567");
        assert_eq!(grouped(u64::MAX), "18.446.744.073.709.551.615");
    }

    #[test]
    fn decimals_use_a_comma_and_drop_zeros() {
        assert_eq!(decimal(12_000.0), "12.000");
        assert_eq!(decimal(1234.5), "1.234,5");
        assert_eq!(decimal(0.25), "0,25");
        assert_eq!(decimal(-2500.126), "-2.500,13");
        assert_eq!(decimal(-0.001), "0");
    }

    #[test]
    fn numbers_that_are_not_finite_stay_readable() {
        assert_eq!(decimal(f64::NAN), "NaN");
        assert_eq!(decimal(f64::INFINITY), "inf");
    }
    use crate::message::{Dice, LossReason, Message};

    const HALF_MIA: Rules = Rules { half_mia: true };

    fn pair(high: u8, low: u8) -> Dice {
        Dice { high, low }
    }

    #[test]
    fn mia_is_21_without_half_mia() {
        assert_eq!(dice(pair(2, 1), Rules::default()), "Mäxchen!");
        assert_eq!(dice(pair(4, 2), Rules::default()), "42");
    }

    #[test]
    fn mia_is_42_and_21_is_half_mia_with_half_mia() {
        assert_eq!(dice(pair(4, 2), HALF_MIA), "Mäxchen!");
        assert_eq!(dice(pair(2, 1), HALF_MIA), "Halbes Mäxchen!");
    }

    #[test]
    fn pairs_and_plain_dice_are_named() {
        assert_eq!(dice(pair(5, 5), Rules::default()), "Fünferpasch");
        assert_eq!(dice(pair(6, 3), Rules::default()), "63");
    }

    #[test]
    fn names_are_joined_the_german_way() {
        let names = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(join(&names(&[])), "");
        assert_eq!(join(&names(&["anna"])), "anna");
        assert_eq!(join(&names(&["anna", "bert"])), "anna und bert");
        assert_eq!(
            join(&names(&["anna", "bert", "carla"])),
            "anna, bert und carla"
        );
    }

    #[test]
    fn one_loser_loses_and_several_lose() {
        let lost = |players: &[&str]| Message::PlayerLost {
            players: players.iter().map(ToString::to_string).collect(),
            reason: LossReason::DidNotAnnounce,
        };
        assert_eq!(
            describe(&lost(&["anna"]), Rules::default()).as_deref(),
            Some("anna verliert: hat nichts angesagt")
        );
        assert_eq!(
            describe(&lost(&["anna", "bert"]), Rules::default()).as_deref(),
            Some("anna und bert verlieren: hat nichts angesagt")
        );
    }

    #[test]
    fn scores_are_not_described() {
        assert_eq!(describe(&Message::Score(vec![]), Rules::default()), None);
    }

    #[test]
    fn every_pair_has_its_name() {
        let named: Vec<String> = (1..=6)
            .map(|face| dice(pair(face, face), Rules::default()))
            .collect();

        assert_eq!(
            named,
            [
                "Einserpasch",
                "Zweierpasch",
                "Dreierpasch",
                "Viererpasch",
                "Fünferpasch",
                "Sechserpasch"
            ]
        );
    }

    #[test]
    fn every_loss_has_a_reason_in_words() {
        let reasons = [
            (
                LossReason::SeeBeforeFirstRoll,
                "wollte als erster am Zug sehen",
            ),
            (
                LossReason::LiedAboutMia,
                "wer Mäxchen sagt, sollte Mäxchen haben",
            ),
            (
                LossReason::AnnouncedLosingDice,
                "man muss schon mehr sagen als vorher angesagt war",
            ),
            (LossReason::DidNotAnnounce, "hat nichts angesagt"),
            (LossReason::DidNotTakeTurn, "hat keinen Spielzug gemacht"),
            (
                LossReason::InvalidTurn,
                "wollte einen ungültigen Spielzug machen",
            ),
            (
                LossReason::SeeFailed,
                "wollte sehen... aber doch nicht sowas!",
            ),
            (
                LossReason::CaughtBluffing,
                "hatte gehofft, dass das keiner überprüft",
            ),
            (LossReason::Mia, "Mäxchen"),
            (LossReason::Other("NEW_REASON".to_string()), "NEW_REASON"),
        ];

        for (reason, words) in reasons {
            assert_eq!(loss_reason(&reason), words);
        }
    }

    #[test]
    fn every_cancellation_has_a_reason_in_words() {
        assert_eq!(
            cancel_reason(&CancelReason::NoPlayers),
            "niemand wollte mitspielen"
        );
        assert_eq!(
            cancel_reason(&CancelReason::OnlyOnePlayer),
            "alleine kann man nicht Mäxchen spielen"
        );
        assert_eq!(cancel_reason(&CancelReason::Other("X".to_string())), "X");
    }

    #[test]
    fn every_message_of_a_round_is_told_in_one_line() {
        let anna = || "anna".to_string();
        let rules = Rules::default();
        let lines = [
            (
                Message::RoundStarted {
                    number: 1,
                    players: vec![anna(), "bert".to_string()],
                    rules: None,
                },
                "Teilnehmer: anna und bert",
            ),
            (
                Message::RoundCanceled(CancelReason::NoPlayers),
                "Runde abgebrochen: niemand wollte mitspielen",
            ),
            (Message::PlayerRolls(anna()), "anna würfelt..."),
            (
                Message::PlayerBlindRolls(anna()),
                "anna würfelt ohne zu gucken...",
            ),
            (Message::PlayerWantsToSee(anna()), "anna will sehen!"),
            (
                Message::Announced {
                    player: anna(),
                    dice: pair(2, 1),
                },
                "anna sagt an: Mäxchen!",
            ),
            (
                Message::ActualDice(pair(3, 3)),
                "Die Würfel werden aufgedeckt: Dreierpasch",
            ),
        ];

        for (message, line) in lines {
            assert_eq!(describe(&message, rules).as_deref(), Some(line));
        }
    }
}
