use core::{net::SocketAddr, time::Duration};
use std::io;

use tokio::{
    net::UdpSocket,
    signal,
    sync::mpsc::{self, UnboundedSender},
    time::{self, MissedTickBehavior},
};

use crate::{
    dealer::Dealer,
    game::{Effect, Event, Game},
    prelude::ClientCommand,
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
const MAX_DATAGRAM_SIZE: usize = 2048;

/// Drives a [`Game`] with UDP datagrams, timers and heartbeats until Ctrl-C.
///
/// ```no_run
/// use std::time::Duration;
/// use maexchen::{Game, RandomDealer, serve};
/// use tokio::net::UdpSocket;
///
/// # async fn run() -> std::io::Result<()> {
/// let socket = UdpSocket::bind("0.0.0.0:9000").await?;
/// let game = Game::new(RandomDealer, Duration::from_millis(250), true);
/// serve(socket, game, Duration::ZERO).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Fails when receiving from the socket fails.
pub async fn serve<D: Dealer>(
    socket: UdpSocket,
    mut game: Game<D>,
    initial_delay: Duration,
) -> io::Result<Game<D>> {
    let (events, mut queued_events) = mpsc::unbounded_channel();
    schedule(&events, initial_delay, Event::Start);

    let mut heartbeat = time::interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut buffer = [0u8; MAX_DATAGRAM_SIZE];
    loop {
        let event = tokio::select! {
            received = socket.recv_from(&mut buffer) => {
                let (size, from) = received?;
                match parse_datagram(&buffer[..size], from) {
                    Some(event) => event,
                    None => continue,
                }
            }
            _ = heartbeat.tick() => Event::Heartbeat,
            Some(event) = queued_events.recv() => event,
            _ = signal::ctrl_c() => return Ok(game),
        };

        for effect in game.handle(event) {
            apply(effect, &socket, &events).await;
        }
    }
}

/// Shows a message of the protocol the way the Python client logs it: the command followed by
/// the parameters in quotes and brackets, as in `ROUND STARTED ['5', 'anna,bert']`.
fn readable(message: &str) -> String {
    let mut parts = message.split(';');
    let command = parts.next().unwrap_or_default();
    let parameters: Vec<String> = parts.map(|parameter| format!("'{parameter}'")).collect();
    format!("{command} [{}]", parameters.join(", "))
}

fn parse_datagram(datagram: &[u8], from: SocketAddr) -> Option<Event> {
    let received = String::from_utf8_lossy(datagram);
    let line = received.trim_end_matches(['\r', '\n']);
    log::debug!(target: "receive", "<---- {} from {from}", readable(line));

    match line.parse::<ClientCommand>() {
        Ok(command) => Some(Event::Received { from, command }),
        Err(error) => {
            log::debug!(target: "receive", "ignoring {} from {from}: {error}", readable(line));
            None
        }
    }
}

async fn apply(effect: Effect, socket: &UdpSocket, events: &UnboundedSender<Event>) {
    match effect {
        Effect::Send { to, message } => {
            log::log!(
                target: "send",
                message.loglevel(),
                "----> {} to {to}",
                readable(&message.to_string()),
            );
            if let Err(error) = socket.send_to(message.to_string().as_bytes(), to).await {
                log::warn!(target: "send", "could not send to {to}: {error}");
            }
        }
        Effect::StartTimer { id, after } => schedule(events, after, Event::TimerFired(id)),
    }
}

fn schedule(events: &UnboundedSender<Event>, after: Duration, event: Event) {
    let events = events.clone();
    tokio::spawn(async move {
        time::sleep(after).await;
        let _ = events.send(event);
    });
}

#[cfg(test)]
mod tests {
    use super::readable;

    #[test]
    fn parameters_are_quoted_and_put_in_brackets() {
        assert_eq!(
            readable("ROUND STARTED;5;anna,bert"),
            "ROUND STARTED ['5', 'anna,bert']"
        );
    }

    #[test]
    fn a_message_without_parameters_has_empty_brackets() {
        assert_eq!(readable("HEARTBEAT"), "HEARTBEAT []");
        assert_eq!(readable(""), " []");
    }
}
