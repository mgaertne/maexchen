#![warn(clippy::pedantic)]

use core::time::Duration;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use log::LevelFilter;
use log4rs::{
    Config,
    append::console::ConsoleAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
};
use maexchen::{BlindRoll, Game, MiaRule, RandomDealer, RuleSet, Rules, serve};
use tokio::net::UdpSocket;

/// Maexchen server
#[derive(Clone, Parser, Debug)]
#[command(version, about, long_about = None)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "every switch of the command line has a negated twin"
)]
struct CommandLineOptions {
    /// the server port to listen on
    #[arg(short, long, default_value_t = 9000)]
    port: u16,
    /// initial delay (ms)
    #[arg(short = 'd', long, default_value_t = 0)]
    initial_delay: u64,
    /// start rounds early (default)
    #[arg(
        group = "start_early",
        long,
        default_value_t = true,
        overrides_with = "no_start_rounds_early"
    )]
    start_rounds_early: bool,
    /// do not start rounds early
    #[arg(group = "start_early", long, default_value_t = false)]
    no_start_rounds_early: bool,
    /// default timeout for client reactions (ms)
    #[arg(long, short, default_value_t = 250)]
    answer_timeout: u64,
    /// the rules of the rounds
    #[arg(long, value_enum, default_value_t)]
    rules: RulesChoice,
    /// allow blind rolls
    #[arg(long, default_value_t = false, overrides_with = "with_no_blind_roll")]
    with_blind_roll: bool,
    /// forbid blind rolls (default)
    #[arg(long, default_value_t = false, overrides_with = "with_blind_roll")]
    with_no_blind_roll: bool,
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

/// Which rules the rounds are played with.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum RulesChoice {
    /// 2,1 is the Mia
    #[default]
    Mia21,
    /// 4,2 is the Mia and 2,1 the half Mia
    Mia42,
    /// the rules change from round to round and are announced
    Random,
}

impl CommandLineOptions {
    fn rounds_start_early(&self) -> bool {
        self.start_rounds_early && !self.no_start_rounds_early
    }

    fn rule_set(&self) -> RuleSet {
        let blind_roll = if self.with_blind_roll && !self.with_no_blind_roll {
            BlindRoll::Allowed
        } else {
            BlindRoll::Forbidden
        };
        match self.rules {
            RulesChoice::Mia21 => RuleSet::Fixed(Rules::new(MiaRule::Mia21, blind_roll)),
            RulesChoice::Mia42 => RuleSet::Fixed(Rules::new(MiaRule::Mia42, blind_roll)),
            RulesChoice::Random => RuleSet::Random { blind_roll },
        }
    }
}

fn configure_logging<V: Into<LevelFilter>>(verbosity_level: V) -> Result<()> {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S,%3f)} | {l:<8} | {t:<20} | {h({m})}{n}",
        )))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            Root::builder()
                .appender("stdout")
                .build(verbosity_level.into()),
        )?;

    log4rs::init_config(config)?;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = CommandLineOptions::parse();

    configure_logging(args.verbosity)?;

    let socket = UdpSocket::bind(("0.0.0.0", args.port)).await?;
    log::info!(
        target: "server",
        "Mia server started on port {} with initial delay {}",
        args.port,
        args.initial_delay
    );

    let rule_set = args.rule_set();
    log::info!(target: "server", "Playing with {rule_set:?}");
    let game = Game::new(
        RandomDealer,
        Duration::from_millis(args.answer_timeout),
        args.rounds_start_early(),
    )
    .with_rule_set(rule_set);
    serve(socket, game, Duration::from_millis(args.initial_delay)).await?;

    Ok(())
}

#[cfg(test)]
mod command_line_tests {
    use clap::Parser;
    use maexchen::{BlindRoll, MiaRule, RuleSet, Rules};
    use rstest::rstest;

    use super::CommandLineOptions;

    fn options(arguments: &[&str]) -> CommandLineOptions {
        CommandLineOptions::try_parse_from(["maexchen_server"].iter().chain(arguments))
            .expect("the arguments are valid")
    }

    fn rounds_start_early(arguments: &[&str]) -> bool {
        options(arguments).rounds_start_early()
    }

    #[rstest]
    #[case(&[], RuleSet::Fixed(Rules::new(MiaRule::Mia21, BlindRoll::Forbidden)))]
    #[case(&["--rules", "mia42"], RuleSet::Fixed(Rules::new(MiaRule::Mia42, BlindRoll::Forbidden)))]
    #[case(&["--rules", "mia42", "--with-blind-roll"], RuleSet::Fixed(Rules::new(MiaRule::Mia42, BlindRoll::Allowed)))]
    #[case(&["--with-blind-roll"], RuleSet::Fixed(Rules::new(MiaRule::Mia21, BlindRoll::Allowed)))]
    #[case(&["--rules", "random"], RuleSet::Random { blind_roll: BlindRoll::Forbidden })]
    #[case(&["--rules", "random", "--with-blind-roll"], RuleSet::Random { blind_roll: BlindRoll::Allowed })]
    #[case(&["--with-blind-roll", "--with-no-blind-roll"], RuleSet::Fixed(Rules::new(MiaRule::Mia21, BlindRoll::Forbidden)))]
    #[case(&["--with-no-blind-roll", "--with-blind-roll"], RuleSet::Fixed(Rules::new(MiaRule::Mia21, BlindRoll::Allowed)))]
    fn the_options_decide_the_rule_set(#[case] arguments: &[&str], #[case] expected: RuleSet) {
        assert_eq!(options(arguments).rule_set(), expected);
    }

    #[test]
    fn unknown_rules_are_rejected() {
        assert!(CommandLineOptions::try_parse_from(["maexchen_server", "--rules", "21"]).is_err());
    }

    #[test]
    fn rounds_start_early_by_default() {
        assert!(rounds_start_early(&[]));
    }

    #[test]
    fn no_start_rounds_early_turns_it_off() {
        assert!(!rounds_start_early(&["--no-start-rounds-early"]));
    }
}
