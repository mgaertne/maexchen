//! Command line front end of the spectator; the window lives in `maexchen_spectator_core`.

use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use anyhow::{Result, anyhow};
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use log::LevelFilter;
use log4rs::{
    Config,
    append::console::ConsoleAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
};
use maexchen_spectator_core::{
    app,
    connection::{self, Endpoint},
};

/// Shows the scores and the rounds of a Maexchen server.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CommandLineOptions {
    /// the server ip to connect to
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,
    /// the server port to connect to
    #[arg(short, long, default_value_t = 9000)]
    port: u16,
    /// the name to register with
    #[arg(short, long, default_value = "spectator")]
    name: String,
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
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

    let (sender, receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let endpoint = Endpoint {
        host: args.ip.clone(),
        port: args.port,
        name: args.name,
    };
    let connection = connection::spawn(endpoint, sender, Arc::clone(&stop));

    let window = app::run(format!("{}:{}", args.ip, args.port), receiver);

    stop.store(true, Ordering::Release);
    let joined = connection.join();
    window.map_err(|error| anyhow!("the window failed: {error}"))?;
    joined.map_err(|_| anyhow!("the connection thread panicked"))
}
