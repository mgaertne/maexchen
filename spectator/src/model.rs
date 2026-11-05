//! What a spectator knows about the game, built up from the messages of the server.

use core::time::Duration;
use std::collections::{BTreeMap, VecDeque};

use crate::message::{CancelReason, Dice, LossReason, Message};

const MAX_EVENTS: usize = 200;
/// How often the scores are sampled unless set otherwise, as the Java spectator did.
pub const DEFAULT_SAMPLE_INTERVAL: Duration = Duration::from_secs(10);
/// Twelve hours of samples; older ones are dropped so that a long session stays fast.
const MAX_SAMPLES: usize = 12 * 60 * 6;

/// How a round ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Somebody lost; the round counts for the others.
    Lost {
        /// The players that lost.
        players: Vec<String>,
        /// Why they lost.
        reason: LossReason,
    },
    /// The round was canceled before the first turn.
    Canceled(CancelReason),
}

/// The round that is running or was played last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round {
    /// Number of the round.
    pub number: u64,
    /// The players in turn order.
    pub players: Vec<String>,
    /// The player whose action came last.
    pub acting: Option<String>,
    /// The last announcement and who made it.
    pub announced: Option<(String, Dice)>,
    /// The dice that were revealed, if any.
    pub revealed: Option<Dice>,
    /// How the round ended; `None` while it runs.
    pub outcome: Option<Outcome>,
}

/// A message together with the time it arrived, counted from the start of the spectator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// When the message arrived.
    pub at: Duration,
    /// The message itself.
    pub message: Message,
}

/// The state shown by the spectator.
#[derive(Debug)]
pub struct GameView {
    scores: Vec<(String, i32)>,
    scores_round: Option<u64>,
    round: Option<Round>,
    history: BTreeMap<String, Vec<(Duration, i32)>>,
    events: VecDeque<Event>,
    last_sample: Option<Duration>,
    sample_interval: Duration,
}

impl Default for GameView {
    fn default() -> Self {
        Self {
            scores: Vec::new(),
            scores_round: None,
            round: None,
            history: BTreeMap::new(),
            events: VecDeque::new(),
            last_sample: None,
            sample_interval: DEFAULT_SAMPLE_INTERVAL,
        }
    }
}

impl GameView {
    /// Takes a message that arrived `at` after the start of the spectator into account.
    pub fn apply(&mut self, message: Message, at: Duration) {
        self.update(&message, at);
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(Event { at, message });
    }

    /// Changes how often the scores are sampled for the history; it counts from now on.
    pub fn set_sample_interval(&mut self, interval: Duration) {
        self.sample_interval = interval;
    }

    /// The scores, sorted by player name as the server sends them.
    #[must_use]
    pub fn scores(&self) -> &[(String, i32)] {
        &self.scores
    }

    /// The number of the round after which the scores were sent.
    #[must_use]
    pub fn scores_round(&self) -> Option<u64> {
        self.scores_round
    }

    /// The running round, or the last one that was played.
    #[must_use]
    pub fn round(&self) -> Option<&Round> {
        self.round.as_ref()
    }

    /// The most recent messages, oldest first.
    pub fn events(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }

    /// The scores of every player over time, sampled at most as often as the sample interval allows.
    #[must_use]
    pub fn history(&self) -> &BTreeMap<String, Vec<(Duration, i32)>> {
        &self.history
    }

    /// The points every player earned per minute during the last `window`.
    #[must_use]
    pub fn points_per_minute(&self, window: Duration) -> Vec<(String, f64)> {
        self.history
            .iter()
            .map(|(name, samples)| {
                (
                    name.clone(),
                    rate_at(samples, self.last_sample.unwrap_or_default(), window).unwrap_or(0.0),
                )
            })
            .collect()
    }

    /// The points per minute at every sample after the first, each looking back over `window`.
    #[must_use]
    pub fn points_per_minute_series(
        &self,
        window: Duration,
    ) -> BTreeMap<String, Vec<(Duration, f64)>> {
        self.history
            .iter()
            .map(|(name, samples)| {
                let rates = samples
                    .iter()
                    .filter_map(|(at, _)| Some((*at, rate_at(samples, *at, window)?)))
                    .collect();
                (name.clone(), rates)
            })
            .collect()
    }

    fn sample_is_due(&self, at: Duration) -> bool {
        self.last_sample
            .is_none_or(|last| at.saturating_sub(last) >= self.sample_interval)
    }

    fn update(&mut self, message: &Message, at: Duration) {
        match message {
            Message::RoundStarted {
                number, players, ..
            } => {
                self.round = Some(Round {
                    number: *number,
                    players: players.clone(),
                    acting: None,
                    announced: None,
                    revealed: None,
                    outcome: None,
                });
            }
            Message::RoundCanceled(reason) => {
                if let Some(round) = self.round.as_mut() {
                    round.outcome = Some(Outcome::Canceled(reason.clone()));
                }
            }
            Message::PlayerRolls(player)
            | Message::PlayerBlindRolls(player)
            | Message::PlayerWantsToSee(player) => {
                if let Some(round) = self.round.as_mut() {
                    round.acting = Some(player.clone());
                }
            }
            Message::Announced { player, dice } => {
                if let Some(round) = self.round.as_mut() {
                    round.announced = Some((player.clone(), *dice));
                }
            }
            Message::ActualDice(dice) => {
                if let Some(round) = self.round.as_mut() {
                    round.revealed = Some(*dice);
                }
            }
            Message::PlayerLost { players, reason } => {
                if let Some(round) = self.round.as_mut() {
                    round.outcome = Some(Outcome::Lost {
                        players: players.clone(),
                        reason: reason.clone(),
                    });
                }
            }
            Message::Score(scores) => {
                self.scores.clone_from(scores);
                self.scores_round = self.round.as_ref().map(|round| round.number);
                if self.sample_is_due(at) {
                    self.last_sample = Some(at);
                    for (name, points) in scores {
                        let samples = self.history.entry(name.clone()).or_default();
                        samples.push((at, *points));
                        let surplus = samples.len().saturating_sub(MAX_SAMPLES);
                        samples.drain(..surplus);
                    }
                }
            }
        }
    }
}

fn rate_at(samples: &[(Duration, i32)], at: Duration, window: Duration) -> Option<f64> {
    let &(first_at, first_points) = samples.first()?;
    let from = at.saturating_sub(window).max(first_at);
    let span = at.saturating_sub(from);
    if span.is_zero() {
        return None;
    }
    let points_at = |moment: Duration| {
        let known = samples.partition_point(|(sampled, _)| *sampled <= moment);
        known
            .checked_sub(1)
            .map_or(first_points, |last| samples[last].1)
    };
    Some(f64::from(points_at(at) - points_at(from)) / (span.as_secs_f64() / 60.0))
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::{GameView, Outcome};
    use crate::message::{CancelReason, Dice, LossReason, Message};

    fn seconds(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    fn started(players: &[&str]) -> Message {
        Message::RoundStarted {
            number: 1,
            players: players.iter().map(ToString::to_string).collect(),
            rules: None,
        }
    }

    fn score(entries: &[(&str, i32)]) -> Message {
        Message::Score(entries.iter().map(|(n, p)| (n.to_string(), *p)).collect())
    }

    #[test]
    fn a_canceled_round_keeps_its_reason() {
        let mut view = GameView::default();
        view.apply(Message::RoundCanceled(CancelReason::NoPlayers), seconds(1));
        assert!(view.round().is_none());

        view.apply(started(&["anna"]), seconds(2));
        view.apply(
            Message::RoundCanceled(CancelReason::OnlyOnePlayer),
            seconds(3),
        );

        let round = view.round().expect("a round was started");
        assert_eq!(
            round.outcome,
            Some(Outcome::Canceled(CancelReason::OnlyOnePlayer))
        );
    }

    #[test]
    fn a_round_collects_what_happens_in_it() {
        let mut view = GameView::default();
        view.apply(started(&["anna", "bert"]), seconds(1));
        view.apply(Message::PlayerRolls("anna".to_string()), seconds(2));
        view.apply(
            Message::Announced {
                player: "anna".to_string(),
                dice: Dice { high: 6, low: 5 },
            },
            seconds(3),
        );
        view.apply(Message::ActualDice(Dice { high: 3, low: 1 }), seconds(4));
        view.apply(
            Message::PlayerLost {
                players: vec!["anna".to_string()],
                reason: LossReason::CaughtBluffing,
            },
            seconds(5),
        );

        let round = view.round().expect("a round was started");
        assert_eq!(round.acting.as_deref(), Some("anna"));
        assert_eq!(round.revealed, Some(Dice { high: 3, low: 1 }));
        assert_eq!(
            round.outcome,
            Some(Outcome::Lost {
                players: vec!["anna".to_string()],
                reason: LossReason::CaughtBluffing,
            })
        );
    }

    #[test]
    fn a_new_round_replaces_the_old_one() {
        let mut view = GameView::default();
        view.apply(started(&["anna", "bert"]), seconds(1));
        view.apply(Message::PlayerRolls("anna".to_string()), seconds(2));
        view.apply(started(&["bert", "anna"]), seconds(3));

        let round = view.round().expect("a round was started");
        assert_eq!(round.players, ["bert", "anna"]);
        assert_eq!(round.acting, None);
    }

    #[test]
    fn scores_are_replaced_and_remembered() {
        let mut view = GameView::default();
        view.apply(score(&[("anna", 0)]), seconds(0));
        view.apply(score(&[("anna", 2), ("bert", 1)]), seconds(60));

        assert_eq!(
            view.scores(),
            [("anna".to_string(), 2), ("bert".to_string(), 1)]
        );
        assert_eq!(view.history()["anna"], [(seconds(0), 0), (seconds(60), 2)]);
    }

    #[test]
    fn points_per_minute_look_at_the_window_only() {
        let mut view = GameView::default();
        view.apply(score(&[("anna", 0)]), seconds(0));
        view.apply(score(&[("anna", 10)]), seconds(60));
        view.apply(score(&[("anna", 14)]), seconds(120));

        assert_eq!(
            view.points_per_minute(seconds(60)),
            [("anna".to_string(), 4.0)]
        );
    }

    #[test]
    fn points_per_minute_use_the_time_played_if_the_window_is_longer() {
        let mut view = GameView::default();
        view.apply(score(&[("anna", 0)]), seconds(0));
        view.apply(score(&[("anna", 6)]), seconds(120));

        assert_eq!(
            view.points_per_minute(seconds(600)),
            [("anna".to_string(), 3.0)]
        );
    }

    #[test]
    fn a_spectator_that_joins_late_does_not_count_the_points_from_before() {
        let mut view = GameView::default();
        view.apply(score(&[("anna", 6000)]), seconds(5));
        view.apply(score(&[("anna", 6010)]), seconds(15));
        view.apply(score(&[("anna", 6010)]), seconds(25));

        assert_eq!(
            view.points_per_minute_series(seconds(300))["anna"],
            [(seconds(15), 60.0), (seconds(25), 30.0)]
        );
        assert_eq!(
            view.points_per_minute(seconds(300)),
            [("anna".to_string(), 30.0)]
        );
    }

    #[test]
    fn scores_that_follow_each_other_quickly_are_sampled_once() {
        let mut view = GameView::default();
        for tenth in 0..1000 {
            view.apply(
                score(&[("anna", tenth)]),
                Duration::from_millis(u64::try_from(tenth).unwrap() * 100),
            );
        }

        assert_eq!(view.scores(), [("anna".to_string(), 999)]);
        assert_eq!(view.history()["anna"].len(), 10);
    }

    #[test]
    fn the_sample_interval_can_be_changed() {
        let mut view = GameView::default();
        view.set_sample_interval(seconds(1));
        for second in 0..100 {
            view.apply(score(&[("anna", 1)]), seconds(second));
        }

        assert_eq!(view.history()["anna"].len(), 100);
    }

    #[test]
    fn the_history_is_limited() {
        let mut view = GameView::default();
        for index in 0..5000_u64 {
            view.apply(score(&[("anna", 1)]), seconds(index * 10));
        }

        assert_eq!(view.history()["anna"].len(), 4320);
        assert_eq!(view.history()["anna"][0].0, seconds(680 * 10));
    }

    #[test]
    fn the_series_has_one_rate_per_score_message() {
        let mut view = GameView::default();
        view.apply(score(&[("anna", 0)]), seconds(0));
        view.apply(score(&[("anna", 10)]), seconds(60));
        view.apply(score(&[("anna", 14)]), seconds(120));

        assert_eq!(
            view.points_per_minute_series(seconds(60))["anna"],
            [(seconds(60), 10.0), (seconds(120), 4.0)]
        );
    }

    #[test]
    fn the_scores_remember_after_which_round_they_came() {
        let mut view = GameView::default();
        view.apply(score(&[("anna", 0)]), seconds(0));
        assert_eq!(view.scores_round(), None);

        view.apply(started(&["anna", "bert"]), seconds(1));
        view.apply(score(&[("anna", 1)]), seconds(2));
        assert_eq!(view.scores_round(), Some(1));
    }

    #[test]
    fn nothing_is_earned_before_anything_happened() {
        assert_eq!(GameView::default().points_per_minute(seconds(60)), []);
    }

    #[test]
    fn only_the_most_recent_events_are_kept() {
        let mut view = GameView::default();
        for second in 0..250 {
            view.apply(Message::PlayerRolls("anna".to_string()), seconds(second));
        }

        assert_eq!(view.events().count(), 200);
        assert_eq!(
            view.events().next().map(|event| event.at),
            Some(seconds(50))
        );
    }
}
