//! The rules of a round: which dice are the Mia and whether blind rolls are allowed.

use core::{fmt, str::FromStr};

use thiserror::Error;

/// Which dice are the Mia.
///
/// ```
/// use maexchen::MiaRule;
///
/// assert_eq!(MiaRule::Mia42.to_string(), "mia42");
/// assert_eq!("mia21".parse(), Ok(MiaRule::Mia21));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum MiaRule {
    /// 2,1 is the Mia and there is no half Mia (`mia21` on the wire).
    #[default]
    Mia21,
    /// 4,2 is the Mia, and 2,1 is the half Mia that beats everything but the Mia (`mia42`).
    Mia42,
}

impl fmt::Display for MiaRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mia21 => "mia21",
            Self::Mia42 => "mia42",
        })
    }
}

/// Why a text is not a rule.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseRulesError {
    /// The text starts with something other than `mia21` or `mia42`.
    #[error("unknown Mia rule `{0}`, expected `mia21` or `mia42`")]
    UnknownMia(String),
    /// The text continues with something other than `blind`.
    #[error("unknown rule `{0}`, expected `blind`")]
    UnknownOption(String),
}

impl FromStr for MiaRule {
    type Err = ParseRulesError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "mia21" => Ok(Self::Mia21),
            "mia42" => Ok(Self::Mia42),
            other => Err(ParseRulesError::UnknownMia(other.to_string())),
        }
    }
}

/// Whether a player may roll the dice without looking at them.
///
/// ```
/// use maexchen::BlindRoll;
///
/// assert_eq!(BlindRoll::default(), BlindRoll::Forbidden);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlindRoll {
    /// Players must look at the dice they rolled (`BLIND_ROLL` is ignored).
    #[default]
    Forbidden,
    /// Players may roll blind (`,blind` on the wire).
    Allowed,
}

/// The rules a round is played with.
///
/// On the wire they are the Mia rule, followed by `,blind` if blind rolls are allowed: `mia21`,
/// `mia42` or `mia42,blind`.
///
/// ```
/// use maexchen::{BlindRoll, MiaRule, Rules};
///
/// let rules = Rules::new(MiaRule::Mia42, BlindRoll::Allowed);
///
/// assert_eq!(rules.to_string(), "mia42,blind");
/// assert_eq!("mia42,blind".parse(), Ok(rules));
/// assert_eq!(Rules::default().to_string(), "mia21");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rules {
    /// Which dice are the Mia.
    pub mia: MiaRule,
    /// Whether a player may roll without looking.
    pub blind_roll: BlindRoll,
}

impl Rules {
    /// The rules with the given Mia rule and blind rolls.
    #[must_use]
    pub fn new(mia: MiaRule, blind_roll: BlindRoll) -> Self {
        Self { mia, blind_roll }
    }

    /// Whether a player may roll without looking at the dice.
    #[must_use]
    pub fn allows_blind_roll(self) -> bool {
        self.blind_roll == BlindRoll::Allowed
    }
}

impl fmt::Display for Rules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mia)?;
        if self.allows_blind_roll() {
            f.write_str(",blind")?;
        }
        Ok(())
    }
}

impl FromStr for Rules {
    type Err = ParseRulesError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (mia, option) = match text.split_once(',') {
            Some((mia, option)) => (mia, Some(option)),
            None => (text, None),
        };
        let blind_roll = match option {
            None => BlindRoll::Forbidden,
            Some("blind") => BlindRoll::Allowed,
            Some(other) => return Err(ParseRulesError::UnknownOption(other.to_string())),
        };
        Ok(Self::new(mia.parse()?, blind_roll))
    }
}

/// How the rules of the rounds are chosen.
///
/// ```
/// use maexchen::{BlindRoll, MiaRule, RuleSet, Rules};
///
/// let fixed = RuleSet::Fixed(Rules::new(MiaRule::Mia42, BlindRoll::Allowed));
/// let random = RuleSet::Random { blind_roll: BlindRoll::Forbidden };
///
/// assert!(!fixed.announces_rules());
/// assert!(random.announces_rules());
/// assert_eq!(random.resolve(|| true), Rules::new(MiaRule::Mia42, BlindRoll::Forbidden));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleSet {
    /// Every round is played with the same rules, which are not announced.
    Fixed(Rules),
    /// Every round is played with new random rules, which are announced in `ROUND STARTED`.
    Random {
        /// Whether blind rolls may be part of the random rules.
        blind_roll: BlindRoll,
    },
}

impl Default for RuleSet {
    fn default() -> Self {
        Self::Fixed(Rules::default())
    }
}

impl RuleSet {
    /// Whether the rules of a round are announced to the clients.
    #[must_use]
    pub fn announces_rules(self) -> bool {
        matches!(self, Self::Random { .. })
    }

    /// The rules of the next round; `coin` decides the open choices, `true` meaning the Mia is
    /// 4,2 and blind rolls are allowed.
    pub fn resolve(self, mut coin: impl FnMut() -> bool) -> Rules {
        match self {
            Self::Fixed(rules) => rules,
            Self::Random { blind_roll } => {
                let mia = if coin() {
                    MiaRule::Mia42
                } else {
                    MiaRule::Mia21
                };
                let blind_roll = if blind_roll == BlindRoll::Allowed && coin() {
                    BlindRoll::Allowed
                } else {
                    BlindRoll::Forbidden
                };
                Rules::new(mia, blind_roll)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{BlindRoll, MiaRule, ParseRulesError, RuleSet, Rules};

    #[rstest]
    #[case(Rules::new(MiaRule::Mia21, BlindRoll::Forbidden), "mia21")]
    #[case(Rules::new(MiaRule::Mia21, BlindRoll::Allowed), "mia21,blind")]
    #[case(Rules::new(MiaRule::Mia42, BlindRoll::Forbidden), "mia42")]
    #[case(Rules::new(MiaRule::Mia42, BlindRoll::Allowed), "mia42,blind")]
    fn rules_convert_to_and_from_their_wire_form(#[case] rules: Rules, #[case] text: &str) {
        assert_eq!(rules.to_string(), text);
        assert_eq!(text.parse(), Ok(rules));
    }

    #[test]
    fn other_texts_are_not_rules() {
        assert_eq!(
            "7".parse::<Rules>(),
            Err(ParseRulesError::UnknownMia("7".to_string()))
        );
        assert_eq!(
            "21".parse::<Rules>(),
            Err(ParseRulesError::UnknownMia("21".to_string()))
        );
        assert_eq!(
            "mia21,loud".parse::<Rules>(),
            Err(ParseRulesError::UnknownOption("loud".to_string()))
        );
        assert!("".parse::<Rules>().is_err());
        assert!("mia21,".parse::<Rules>().is_err());
    }

    #[test]
    fn the_default_rules_are_the_basic_protocol() {
        assert_eq!(
            Rules::default(),
            Rules::new(MiaRule::Mia21, BlindRoll::Forbidden)
        );
        assert_eq!(
            RuleSet::default(),
            RuleSet::Fixed(Rules::new(MiaRule::Mia21, BlindRoll::Forbidden))
        );
    }

    #[rstest]
    #[case(false, false, MiaRule::Mia21, BlindRoll::Forbidden)]
    #[case(true, false, MiaRule::Mia42, BlindRoll::Forbidden)]
    #[case(true, true, MiaRule::Mia42, BlindRoll::Allowed)]
    fn random_rules_follow_the_coin(
        #[case] first: bool,
        #[case] second: bool,
        #[case] mia: MiaRule,
        #[case] blind_roll: BlindRoll,
    ) {
        let mut flips = [first, second].into_iter();
        let rules = RuleSet::Random {
            blind_roll: BlindRoll::Allowed,
        }
        .resolve(|| flips.next().unwrap());

        assert_eq!(rules, Rules::new(mia, blind_roll));
    }

    #[test]
    fn random_rules_never_allow_blind_rolls_unless_asked_for() {
        let rules = RuleSet::Random {
            blind_roll: BlindRoll::Forbidden,
        }
        .resolve(|| true);

        assert!(!rules.allows_blind_roll());
    }
}
