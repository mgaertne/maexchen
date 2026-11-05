use core::{cmp::Ordering, num::ParseIntError, str::FromStr};

use derive_more::Display;
use thiserror::Error;

use crate::rules::{MiaRule, Rules};

/// Two dice, normalized so that the higher die comes first.
///
/// How much dice are worth depends on the [`Rules`]: normal dice by their digits, every pair
/// above them and the Mia on top. Their text form is the wire format, `"6,1"`.
///
/// ```
/// use core::cmp::Ordering;
/// use maexchen::{Rules, SimpleDice};
///
/// let dice: SimpleDice = "5,3".parse().unwrap();
///
/// assert_eq!(dice, SimpleDice::new(3, 5));
/// assert_eq!(dice.cmp_under(SimpleDice::new(1, 1), Rules::default()), Ordering::Less);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display)]
#[display("{},{}", die1, die2)]
pub struct SimpleDice {
    die1: u8,
    die2: u8,
}

impl SimpleDice {
    const DICE_21: Self = Self { die1: 2, die2: 1 };
    const DICE_42: Self = Self { die1: 4, die2: 2 };

    /// Creates dice from two dice values in any order.
    ///
    /// ```
    /// use maexchen::SimpleDice;
    ///
    /// assert_eq!(SimpleDice::new(1, 6), SimpleDice::new(6, 1));
    /// assert_eq!(SimpleDice::new(1, 6).to_string(), "6,1");
    /// ```
    #[must_use]
    pub fn new(a: u8, b: u8) -> Self {
        if b > a {
            Self { die1: b, die2: a }
        } else {
            Self { die1: a, die2: b }
        }
    }

    /// Rolls two fair dice.
    ///
    /// ```
    /// use maexchen::SimpleDice;
    ///
    /// assert!(SimpleDice::roll().to_string().parse::<SimpleDice>().is_ok());
    /// ```
    #[must_use]
    pub fn roll() -> Self {
        let die1 = rand::random_range(1u8..=6);
        let die2 = rand::random_range(1u8..=6);
        Self::new(die1, die2)
    }

    fn is_double(self) -> bool {
        self.die1 == self.die2
    }

    /// Whether these dice are the Mia under `rules`, the highest dice, which is revealed as soon
    /// as it is announced.
    ///
    /// ```
    /// use maexchen::{BlindRoll, MiaRule, Rules, SimpleDice};
    ///
    /// let mia_21 = Rules::new(MiaRule::Mia21, BlindRoll::Forbidden);
    /// let mia_42 = Rules::new(MiaRule::Mia42, BlindRoll::Forbidden);
    ///
    /// assert!(SimpleDice::new(1, 2).is_mia(mia_21));
    /// assert!(SimpleDice::new(2, 4).is_mia(mia_42));
    /// assert!(!SimpleDice::new(6, 6).is_mia(mia_21));
    /// ```
    #[must_use]
    pub fn is_mia(self, rules: Rules) -> bool {
        self == match rules.mia {
            MiaRule::Mia42 => Self::DICE_42,
            _ => Self::DICE_21,
        }
    }

    fn is_half_mia(self, rules: Rules) -> bool {
        rules.mia == MiaRule::Mia42 && self == Self::DICE_21
    }

    fn value_for_ordering(self, rules: Rules) -> u32 {
        let base = if self.is_mia(rules) {
            300
        } else if self.is_half_mia(rules) {
            200
        } else if self.is_double() {
            100
        } else {
            0
        };
        base + 10 * u32::from(self.die1) + u32::from(self.die2)
    }

    /// Compares what the dice are worth under `rules`.
    ///
    /// ```
    /// use core::cmp::Ordering;
    /// use maexchen::{BlindRoll, MiaRule, Rules, SimpleDice};
    ///
    /// let rules = Rules::new(MiaRule::Mia42, BlindRoll::Forbidden);
    ///
    /// assert_eq!(SimpleDice::new(6, 6).cmp_under(SimpleDice::new(6, 5), rules), Ordering::Greater);
    /// assert_eq!(SimpleDice::new(1, 2).cmp_under(SimpleDice::new(6, 6), rules), Ordering::Greater);
    /// ```
    #[must_use]
    pub fn cmp_under(self, other: Self, rules: Rules) -> Ordering {
        self.value_for_ordering(rules)
            .cmp(&other.value_for_ordering(rules))
    }
}

/// Why a text is not a pair of dice.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseDiceError {
    /// The text is not of the form `<die>,<die>`.
    #[error("malformed dice, expected `<die>,<die>`")]
    Malformed,
    /// A die is not a number.
    #[error("a die is not a number: {0}")]
    NotANumber(#[from] ParseIntError),
    /// A die is not between 1 and 6.
    #[error("dice out of range: {0} {1}")]
    OutOfRange(u8, u8),
}

impl FromStr for SimpleDice {
    type Err = ParseDiceError;

    fn from_str(s: &str) -> Result<Self, ParseDiceError> {
        let (die1_str, die2_str) = s.split_once(',').ok_or(ParseDiceError::Malformed)?;
        let die1: u8 = die1_str.parse()?;
        let die2: u8 = die2_str.parse()?;
        if !(1u8..=6).contains(&die1) || !(1u8..=6).contains(&die2) {
            return Err(ParseDiceError::OutOfRange(die1, die2));
        }

        Ok(Self::new(die1, die2))
    }
}

#[cfg(test)]
mod dice_tests {
    use rstest::rstest;

    use super::{ParseDiceError, SimpleDice};
    use crate::rules::{BlindRoll, MiaRule, Rules};

    #[test]
    fn constructor_buillds_new_dice() {
        let dice = SimpleDice::new(6, 1);
        assert_eq!(dice, SimpleDice { die1: 6, die2: 1 });
    }

    #[test]
    fn order_is_normalized_for_constructor() {
        let dice = SimpleDice::new(1, 6);
        assert_eq!(dice, SimpleDice { die1: 6, die2: 1 });
    }

    #[test]
    fn roll_rolls_dice_in_range() {
        let dice = SimpleDice::roll();
        assert!((1u8..=6).contains(&dice.die1));
        assert!((1u8..=6).contains(&dice.die2));
    }

    const ASCENDING_21: [(u8, u8); 21] = [
        (3, 1),
        (3, 2),
        (4, 1),
        (4, 2),
        (4, 3),
        (5, 1),
        (5, 2),
        (5, 3),
        (5, 4),
        (6, 1),
        (6, 2),
        (6, 3),
        (6, 4),
        (6, 5),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (5, 5),
        (6, 6),
        (2, 1),
    ];

    const ASCENDING_42: [(u8, u8); 21] = [
        (3, 1),
        (3, 2),
        (4, 1),
        (4, 3),
        (5, 1),
        (5, 2),
        (5, 3),
        (5, 4),
        (6, 1),
        (6, 2),
        (6, 3),
        (6, 4),
        (6, 5),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (5, 5),
        (6, 6),
        (2, 1),
        (4, 2),
    ];

    fn cases() -> [(Rules, [(u8, u8); 21]); 2] {
        [
            (
                Rules::new(MiaRule::Mia21, BlindRoll::Forbidden),
                ASCENDING_21,
            ),
            (
                Rules::new(MiaRule::Mia42, BlindRoll::Forbidden),
                ASCENDING_42,
            ),
        ]
    }

    fn ascending(order: [(u8, u8); 21]) -> Vec<SimpleDice> {
        order
            .iter()
            .map(|&(die1, die2)| SimpleDice::new(die1, die2))
            .collect()
    }

    #[test]
    fn dice_are_ordered_from_lowest_to_highest() {
        for (rules, order) in cases() {
            let dice = ascending(order);
            for (low, first) in dice.iter().enumerate() {
                for (high, second) in dice.iter().enumerate() {
                    assert_eq!(
                        first.cmp_under(*second, rules),
                        low.cmp(&high),
                        "{first} compared to {second} under {rules}"
                    );
                }
            }
        }
    }

    #[test]
    fn only_the_highest_dice_is_mia() {
        for (rules, order) in cases() {
            let dice = ascending(order);
            let (mia, others) = dice.split_last().expect("there are dice");

            assert!(mia.is_mia(rules), "{rules}");
            assert!(others.iter().all(|other| !other.is_mia(rules)), "{rules}");
        }
    }

    #[test]
    fn invalid_dice_values_are_rejected() {
        assert_eq!(
            "0,1".parse::<SimpleDice>(),
            Err(ParseDiceError::OutOfRange(0, 1))
        );
        assert_eq!(
            "1,0".parse::<SimpleDice>(),
            Err(ParseDiceError::OutOfRange(1, 0))
        );
        assert_eq!(
            "7,5".parse::<SimpleDice>(),
            Err(ParseDiceError::OutOfRange(7, 5))
        );
        assert_eq!(
            "5,7".parse::<SimpleDice>(),
            Err(ParseDiceError::OutOfRange(5, 7))
        );
    }

    #[test]
    fn dice_that_are_no_numbers_are_rejected() {
        for text in ["-1,1", "1,-1", "a,1", "1,", ",1"] {
            assert!(
                matches!(
                    text.parse::<SimpleDice>(),
                    Err(ParseDiceError::NotANumber(_))
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn invalid_parse_string_is_rejected() {
        assert_eq!("asdf".parse::<SimpleDice>(), Err(ParseDiceError::Malformed));
    }

    #[rstest]
    #[case(SimpleDice::new(3, 1), "3,1")]
    #[case(SimpleDice::new(3, 2), "3,2")]
    #[case(SimpleDice::new(4, 1), "4,1")]
    #[case(SimpleDice::new(4, 2), "4,2")]
    #[case(SimpleDice::new(4, 3), "4,3")]
    #[case(SimpleDice::new(5, 1), "5,1")]
    #[case(SimpleDice::new(5, 2), "5,2")]
    #[case(SimpleDice::new(5, 3), "5,3")]
    #[case(SimpleDice::new(5, 4), "5,4")]
    #[case(SimpleDice::new(6, 1), "6,1")]
    #[case(SimpleDice::new(6, 2), "6,2")]
    #[case(SimpleDice::new(6, 3), "6,3")]
    #[case(SimpleDice::new(6, 4), "6,4")]
    #[case(SimpleDice::new(6, 5), "6,5")]
    #[case(SimpleDice::new(1, 1), "1,1")]
    #[case(SimpleDice::new(2, 2), "2,2")]
    #[case(SimpleDice::new(3, 3), "3,3")]
    #[case(SimpleDice::new(4, 4), "4,4")]
    #[case(SimpleDice::new(5, 5), "5,5")]
    #[case(SimpleDice::new(6, 6), "6,6")]
    #[case(SimpleDice::new(2, 1), "2,1")]
    fn dice_converts_to_and_from_its_string(#[case] dice: SimpleDice, #[case] text: &str) {
        assert_eq!(dice.to_string(), text);
        assert_eq!(text.parse::<SimpleDice>().unwrap(), dice);
    }
}

#[cfg(test)]
mod properties {
    use core::cmp::Ordering;

    use proptest::prelude::*;

    use super::SimpleDice;
    use crate::rules::Rules;

    fn dice() -> impl Strategy<Value = SimpleDice> {
        (1u8..=6, 1u8..=6).prop_map(|(a, b)| SimpleDice::new(a, b))
    }

    proptest! {
        #[test]
        fn the_order_of_the_two_dice_does_not_matter(a in 1u8..=6, b in 1u8..=6) {
            prop_assert_eq!(SimpleDice::new(a, b), SimpleDice::new(b, a));
        }

        #[test]
        fn the_text_form_leads_back_to_the_same_dice(dice in dice()) {
            prop_assert_eq!(dice.to_string().parse::<SimpleDice>(), Ok(dice));
        }

        #[test]
        fn any_two_dice_are_comparable_and_equal_only_if_identical(a in dice(), b in dice()) {
            let rules = Rules::default();
            let ordering = a.cmp_under(b, rules);

            prop_assert_eq!(ordering, b.cmp_under(a, rules).reverse());
            prop_assert_eq!(a == b, ordering == Ordering::Equal);
        }

        #[test]
        fn the_order_is_transitive(a in dice(), b in dice(), c in dice()) {
            let rules = Rules::default();
            if a.cmp_under(b, rules).is_le() && b.cmp_under(c, rules).is_le() {
                prop_assert!(a.cmp_under(c, rules).is_le());
            }
        }

        #[test]
        fn text_is_either_valid_dice_or_an_error_but_never_a_panic(text in any::<String>()) {
            let _ = text.parse::<SimpleDice>();
        }
    }
}
