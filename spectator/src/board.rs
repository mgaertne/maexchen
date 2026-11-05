//! The round shown to the audience, replaced only every few seconds so that it can be read.

use core::time::Duration;

use crate::{
    message::Message,
    texts::{Rules, describe},
};

/// How long a finished round stays on the board before the next one may replace it, unless set
/// otherwise.
pub const DEFAULT_THROTTLE: Duration = Duration::from_secs(10);

/// What happened in one round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoundReport {
    /// Number of the round.
    pub number: u64,
    /// The messages of the round, in the order they arrived.
    pub messages: Vec<Message>,
    /// The rules the server announced for this round, if it does.
    pub rules: Option<Rules>,
}

impl RoundReport {
    /// One line of text per message of the round, with dice named according to the rules the
    /// server announced, or `fallback` if it announced none.
    #[must_use]
    pub fn lines(&self, fallback: Rules) -> Vec<String> {
        let rules = self.rules.unwrap_or(fallback);
        self.messages
            .iter()
            .filter_map(|message| describe(message, rules))
            .collect()
    }
}

/// Collects rounds and decides which finished round is shown.
#[derive(Debug)]
pub struct RoundBoard {
    throttle: Duration,
    running: Option<RoundReport>,
    shown: Option<RoundReport>,
    shown_at: Option<Duration>,
}

impl Default for RoundBoard {
    fn default() -> Self {
        Self {
            throttle: DEFAULT_THROTTLE,
            running: None,
            shown: None,
            shown_at: None,
        }
    }
}

impl RoundBoard {
    /// How long a finished round stays on the board.
    #[must_use]
    pub fn throttle(&self) -> Duration {
        self.throttle
    }

    /// Changes how long a finished round stays on the board; it counts from the next round on.
    pub fn set_throttle(&mut self, throttle: Duration) {
        self.throttle = throttle;
    }

    /// Takes a message that arrived `at` after the start of the spectator into account.
    ///
    /// Messages of a round whose start was missed are ignored.
    pub fn apply(&mut self, message: &Message, at: Duration) {
        if let Message::RoundStarted { number, rules, .. } = message {
            self.running = Some(RoundReport {
                number: *number,
                messages: Vec::new(),
                rules: rules.map(|announced| Rules {
                    half_mia: announced.half_mia,
                }),
            });
        }
        let Some(running) = self.running.as_mut() else {
            return;
        };
        if !matches!(message, Message::Score(_)) {
            running.messages.push(message.clone());
        }

        if matches!(
            message,
            Message::PlayerLost { .. } | Message::RoundCanceled(_)
        ) && let Some(finished) = self.running.take()
            && self.may_replace(at)
        {
            self.shown = Some(finished);
            self.shown_at = Some(at);
        }
    }

    /// The finished round that is on display.
    #[must_use]
    pub fn shown(&self) -> Option<&RoundReport> {
        self.shown.as_ref()
    }

    /// How much of `span` has passed since the round on display was shown, from `0.0` to `1.0`.
    #[must_use]
    pub fn progress(&self, now: Duration, span: Duration) -> f32 {
        self.shown_at.map_or(1.0, |shown_at| {
            (now.saturating_sub(shown_at).as_secs_f32() / span.as_secs_f32()).min(1.0)
        })
    }

    fn may_replace(&self, at: Duration) -> bool {
        self.shown_at
            .is_none_or(|shown_at| at.saturating_sub(shown_at) >= self.throttle)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::RoundBoard;
    use crate::{
        message::{LossReason, Message},
        texts::Rules,
    };

    fn seconds(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    fn started(number: u64) -> Message {
        Message::RoundStarted {
            number,
            players: vec!["anna".to_string(), "bert".to_string()],
            rules: None,
        }
    }

    fn lost() -> Message {
        Message::PlayerLost {
            players: vec!["anna".to_string()],
            reason: LossReason::DidNotAnnounce,
        }
    }

    fn play_round(board: &mut RoundBoard, number: u64, at: u64) {
        board.apply(&started(number), seconds(at));
        board.apply(&Message::PlayerRolls("anna".to_string()), seconds(at));
        board.apply(&lost(), seconds(at));
    }

    #[test]
    fn nothing_is_shown_before_a_round_ends() {
        let mut board = RoundBoard::default();
        board.apply(&started(1), seconds(0));

        assert_eq!(board.shown(), None);
    }

    #[test]
    fn a_finished_round_is_shown_line_by_line() {
        let mut board = RoundBoard::default();
        play_round(&mut board, 1, 0);

        let report = board.shown().expect("the round is over");
        assert_eq!(report.number, 1);
        assert_eq!(
            report.lines(Rules::default()),
            [
                "Teilnehmer: anna und bert",
                "anna würfelt...",
                "anna verliert: hat nichts angesagt"
            ]
        );
    }

    #[test]
    fn the_rules_can_change_after_the_round_was_shown() {
        let mut board = RoundBoard::default();
        board.apply(&started(1), seconds(0));
        board.apply(
            &Message::ActualDice(crate::message::Dice { high: 2, low: 1 }),
            seconds(0),
        );
        board.apply(&lost(), seconds(0));
        let report = board.shown().expect("the round is over");

        assert_eq!(
            report.lines(Rules::default())[1],
            "Die Würfel werden aufgedeckt: Mäxchen!"
        );
        assert_eq!(
            report.lines(Rules { half_mia: true })[1],
            "Die Würfel werden aufgedeckt: Halbes Mäxchen!"
        );
    }

    #[test]
    fn announced_rules_win_over_the_setting_in_every_round() {
        let announcing = |number, half_mia| Message::RoundStarted {
            number,
            players: vec!["anna".to_string(), "bert".to_string()],
            rules: Some(crate::message::AnnouncedRules {
                half_mia,
                blind_roll: false,
            }),
        };
        let dice = Message::ActualDice(crate::message::Dice { high: 2, low: 1 });
        let mut board = RoundBoard::default();
        board.set_throttle(Duration::ZERO);

        board.apply(&announcing(1, true), seconds(0));
        board.apply(&dice, seconds(0));
        board.apply(&lost(), seconds(0));
        assert_eq!(
            board
                .shown()
                .expect("round 1 is over")
                .lines(Rules::default())[1],
            "Die Würfel werden aufgedeckt: Halbes Mäxchen!"
        );

        board.apply(&announcing(2, false), seconds(1));
        board.apply(&dice, seconds(1));
        board.apply(&lost(), seconds(1));
        assert_eq!(
            board
                .shown()
                .expect("round 2 is over")
                .lines(Rules { half_mia: true })[1],
            "Die Würfel werden aufgedeckt: Mäxchen!"
        );
    }

    #[test]
    fn a_round_that_ends_too_soon_is_not_shown() {
        let mut board = RoundBoard::default();
        play_round(&mut board, 1, 0);
        play_round(&mut board, 2, 5);

        assert_eq!(board.shown().map(|report| report.number), Some(1));
    }

    #[test]
    fn the_next_round_is_shown_after_ten_seconds() {
        let mut board = RoundBoard::default();
        play_round(&mut board, 1, 0);
        play_round(&mut board, 2, 10);

        assert_eq!(board.shown().map(|report| report.number), Some(2));
    }

    #[test]
    fn the_time_a_round_stays_can_be_changed() {
        let mut board = RoundBoard::default();
        board.set_throttle(seconds(30));
        play_round(&mut board, 1, 0);
        play_round(&mut board, 2, 20);
        assert_eq!(board.shown().map(|report| report.number), Some(1));

        board.set_throttle(seconds(5));
        play_round(&mut board, 3, 25);
        assert_eq!(board.shown().map(|report| report.number), Some(3));
        assert_eq!(board.throttle(), seconds(5));
    }

    #[test]
    fn a_round_whose_start_was_missed_is_ignored() {
        let mut board = RoundBoard::default();
        board.apply(&Message::PlayerRolls("anna".to_string()), seconds(0));
        board.apply(&lost(), seconds(0));

        assert_eq!(board.shown(), None);
    }

    #[test]
    fn progress_runs_from_the_moment_a_round_is_shown() {
        let mut board = RoundBoard::default();
        assert!((board.progress(seconds(3), seconds(10)) - 1.0).abs() < f32::EPSILON);

        play_round(&mut board, 1, 4);

        assert!(board.progress(seconds(4), seconds(10)).abs() < f32::EPSILON);
        assert!((board.progress(seconds(9), seconds(10)) - 0.5).abs() < f32::EPSILON);
        assert!((board.progress(seconds(60), seconds(10)) - 1.0).abs() < f32::EPSILON);
        assert!((board.progress(seconds(9), seconds(20)) - 0.25).abs() < f32::EPSILON);
    }
}
