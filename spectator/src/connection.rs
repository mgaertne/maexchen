//! Keeps the spectator connected to the server in a thread of its own.

use core::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use std::{
    sync::{Arc, mpsc::Sender},
    thread::{self, JoinHandle},
    time::Instant,
};

use maexchen_client::{
    client::{self, ANSWER_TIMEOUT, ClientError, HEARTBEAT_TIMEOUT, Settings},
    transport::UdpTransport,
};

use crate::{
    bot::SpectatorBot,
    update::{Status, Update},
};

const LONGEST_PAUSE_SECONDS: u64 = 8;
const STABLE_AFTER: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(100);

/// Where and as whom the spectator registers.
#[derive(Clone, Debug)]
pub struct Endpoint {
    /// Host name or IP address of the server.
    pub host: String,
    /// UDP port of the server.
    pub port: u16,
    /// The name to register with.
    pub name: String,
}

/// The pause before the attempt after `failures` failed ones: 1, 2, 4 and then 8 seconds.
#[must_use]
pub fn backoff(failures: u32) -> Duration {
    let seconds = 1_u64
        .checked_shl(failures)
        .map_or(LONGEST_PAUSE_SECONDS, |pause| {
            pause.min(LONGEST_PAUSE_SECONDS)
        });
    Duration::from_secs(seconds)
}

/// Connects in a new thread and keeps reconnecting until `stop` is set; join the thread so that
/// the spectator unregisters.
pub fn spawn(endpoint: Endpoint, updates: Sender<Update>, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::spawn(move || keep_connected(&endpoint, &updates, &stop))
}

fn keep_connected(endpoint: &Endpoint, updates: &Sender<Update>, stop: &AtomicBool) {
    let mut failures = 0;
    while !stop.load(Ordering::Acquire) {
        let _window_is_gone = updates.send(Update::Status(Status::Connected));
        let started = Instant::now();
        let Err(error) = connect_once(endpoint, updates, stop) else {
            return;
        };
        if stop.load(Ordering::Acquire) {
            return;
        }

        if started.elapsed() >= STABLE_AFTER {
            failures = 0;
        }
        let retry_in = backoff(failures);
        failures += 1;
        log::warn!("{error}; trying again in {} s", retry_in.as_secs());
        let _window_is_gone = updates.send(Update::Status(Status::Reconnecting {
            reason: error.to_string(),
            retry_in,
        }));
        pause(retry_in, stop);
    }
}

fn connect_once(
    endpoint: &Endpoint,
    updates: &Sender<Update>,
    stop: &AtomicBool,
) -> Result<(), ClientError> {
    let transport = UdpTransport::connect(&endpoint.host, endpoint.port)?;
    let settings = Settings {
        name: endpoint.name.clone(),
        auto_join: false,
        heartbeat_timeout: Some(HEARTBEAT_TIMEOUT),
        answer_timeout: ANSWER_TIMEOUT,
    };
    client::run(
        &transport,
        &mut SpectatorBot::new(updates.clone()),
        &settings,
        stop,
    )
}

fn pause(duration: Duration, stop: &AtomicBool) {
    let end = Instant::now() + duration;
    while !stop.load(Ordering::Acquire) && Instant::now() < end {
        thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::backoff;

    #[test]
    fn the_pause_doubles_up_to_eight_seconds() {
        let pauses: Vec<u64> = (0..6).map(|failures| backoff(failures).as_secs()).collect();

        assert_eq!(pauses, [1, 2, 4, 8, 8, 8]);
    }

    #[test]
    fn the_pause_stays_at_the_maximum_for_any_number_of_failures() {
        assert_eq!(backoff(u32::MAX), Duration::from_secs(8));
    }
}
