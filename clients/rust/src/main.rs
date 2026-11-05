//! Command line front end of the Maexchen client; the framework lives in `maexchen_client`.

use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
use maexchen_client::{
    bot::Bot,
    client::{self, ANSWER_TIMEOUT, HEARTBEAT_TIMEOUT, Settings},
    random_bot::RandomBot,
    simple_bot::SimpleBot,
    spectator_bot::SpectatorBot,
    transport::UdpTransport,
};

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum BotType {
    #[default]
    Simple,
    Random,
    Spectator,
}

impl BotType {
    fn create(self) -> Box<dyn Bot> {
        match self {
            Self::Simple => Box::new(SimpleBot),
            Self::Random => Box::new(RandomBot),
            Self::Spectator => Box::new(SpectatorBot),
        }
    }
}

/// MaexchenBot
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CommandLineOptions {
    /// the server ip to connect to
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,
    /// the server port to connect to
    #[arg(short, long, default_value_t = 9000)]
    port: u16,
    /// the player's name to use (default: maexchen-rust-bot-NNN)
    #[arg(short, long)]
    name: Option<String>,
    /// automatically join new announced matches (default)
    #[arg(
        group = "auto-join",
        long,
        default_value_t = true,
        overrides_with = "no_auto_join"
    )]
    auto_join: bool,
    /// do not automatically join new announced matches
    #[arg(group = "auto-join", long, default_value_t = false)]
    no_auto_join: bool,
    /// give up when the server sends no heartbeats any more (default)
    #[arg(
        group = "watch-heartbeat",
        long,
        default_value_t = true,
        overrides_with = "no_watch_heartbeat"
    )]
    watch_heartbeat: bool,
    /// do not give up when the server sends no heartbeats any more
    #[arg(group = "watch-heartbeat", long, default_value_t = false)]
    no_watch_heartbeat: bool,
    /// sets the flavor of bot to use
    #[arg(long = "type", short = 't', value_enum, default_value_t)]
    bot_type: BotType,
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

impl CommandLineOptions {
    fn auto_join(&self) -> bool {
        self.auto_join && !self.no_auto_join
    }

    fn watch_heartbeat(&self) -> bool {
        self.watch_heartbeat && !self.no_watch_heartbeat
    }

    fn settings(&self) -> Settings {
        Settings {
            name: self.name.clone().unwrap_or_else(random_bot_name),
            auto_join: self.auto_join(),
            heartbeat_timeout: self.watch_heartbeat().then_some(HEARTBEAT_TIMEOUT),
            answer_timeout: ANSWER_TIMEOUT,
        }
    }
}

fn random_bot_name() -> String {
    format!("maexchen-rust-bot-{}", rand::random_range(100..1000))
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

fn main() -> Result<()> {
    let args = CommandLineOptions::parse();

    configure_logging(args.verbosity)?;

    let stop = Arc::new(AtomicBool::new(false));
    let ctrl_c = Arc::clone(&stop);
    ctrlc::set_handler(move || ctrl_c.store(true, Ordering::Release))?;

    let transport = UdpTransport::connect(&args.ip, args.port)?;
    let mut bot = args.bot_type.create();
    client::run(&transport, bot.as_mut(), &args.settings(), &stop)?;
    Ok(())
}

#[cfg(test)]
mod command_line_tests {
    use clap::Parser;

    use super::CommandLineOptions;

    fn options(arguments: &[&str]) -> CommandLineOptions {
        CommandLineOptions::try_parse_from(["maexchen_bot"].iter().chain(arguments))
            .expect("the arguments are valid")
    }

    #[test]
    fn joins_and_watches_heartbeats_by_default() {
        let options = options(&[]);

        assert!(options.auto_join());
        assert!(options.watch_heartbeat());
    }

    #[test]
    fn the_negated_flags_turn_the_behaviour_off() {
        let options = options(&["--no-auto-join", "--no-watch-heartbeat"]);

        assert!(!options.auto_join());
        assert!(!options.watch_heartbeat());
        assert!(options.settings().heartbeat_timeout.is_none());
    }

    #[test]
    fn invents_a_name_unless_one_is_given() {
        assert!(
            options(&[])
                .settings()
                .name
                .starts_with("maexchen-rust-bot-")
        );
        assert_eq!(options(&["-n", "anna"]).settings().name, "anna");
    }

    #[test]
    fn rejects_unknown_bot_types() {
        assert!(CommandLineOptions::try_parse_from(["maexchen_bot", "-t", "nonsense"]).is_err());
    }
}
