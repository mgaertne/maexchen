use rand::seq::SliceRandom;
use uuid::Uuid;

use crate::{RuleSet, Rules, SimpleDice};

/// Source of everything random in a game: dice, turn order and tokens.
///
/// Implement it to play a game with known dice, for example in tests:
///
/// ```
/// use maexchen::{Dealer, SimpleDice};
///
/// struct SnakeEyes;
///
/// impl Dealer for SnakeEyes {
///     fn roll(&mut self) -> SimpleDice {
///         SimpleDice::new(1, 1)
///     }
///
///     fn token(&mut self) -> String {
///         "token".to_string()
///     }
///
///     fn shuffle(&mut self, players: &mut [String]) {
///         players.sort();
///     }
/// }
///
/// assert_eq!(SnakeEyes.roll(), SimpleDice::new(1, 1));
/// ```
pub trait Dealer {
    /// Rolls the dice for a player.
    fn roll(&mut self) -> SimpleDice;
    /// Creates a token that ties an answer to the question it answers.
    fn token(&mut self) -> String;
    /// Brings the players of a round into turn order.
    fn shuffle(&mut self, players: &mut [String]);
    /// Flips a coin for the open choices of random rules; by default always `false`.
    fn coin(&mut self) -> bool {
        false
    }
    /// Decides the rules of the next round from the rule set.
    fn rules(&mut self, rule_set: RuleSet) -> Rules {
        rule_set.resolve(|| self.coin())
    }
}

/// Deals with fair dice, random tokens and a random turn order.
#[derive(Debug, Default)]
pub struct RandomDealer;

impl Dealer for RandomDealer {
    fn roll(&mut self) -> SimpleDice {
        SimpleDice::roll()
    }

    fn token(&mut self) -> String {
        Uuid::new_v4().to_string()
    }

    fn shuffle(&mut self, players: &mut [String]) {
        players.shuffle(&mut rand::rng());
    }

    fn coin(&mut self) -> bool {
        rand::random()
    }
}

#[cfg(test)]
mod tests {
    use super::{Dealer, RandomDealer};

    #[test]
    fn tokens_are_unique() {
        let mut dealer = RandomDealer;

        assert_ne!(dealer.token(), dealer.token());
    }

    #[test]
    fn rolled_dice_are_valid() {
        let mut dealer = RandomDealer;

        assert!((0..100).all(|_| {
            dealer
                .roll()
                .to_string()
                .parse::<crate::SimpleDice>()
                .is_ok()
        }));
    }

    #[test]
    fn the_coin_shows_both_sides_over_time() {
        let mut dealer = RandomDealer;
        let flips: Vec<bool> = (0..200).map(|_| dealer.coin()).collect();

        assert!(flips.contains(&true) && flips.contains(&false));
    }

    #[test]
    fn a_dealer_without_a_coin_always_flips_false() {
        struct Plain;
        impl Dealer for Plain {
            fn roll(&mut self) -> crate::SimpleDice {
                RandomDealer.roll()
            }

            fn token(&mut self) -> String {
                String::new()
            }

            fn shuffle(&mut self, _players: &mut [String]) {}
        }

        assert!(!Plain.coin());
    }

    #[test]
    fn shuffling_keeps_all_players() {
        let mut dealer = RandomDealer;
        let mut players: Vec<String> = ["anna", "bert", "cleo"].map(String::from).to_vec();

        dealer.shuffle(&mut players);
        players.sort();

        assert_eq!(players, ["anna", "bert", "cleo"]);
    }
}
