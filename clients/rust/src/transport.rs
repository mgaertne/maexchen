//! How messages get to and from the server.

use std::{io, net::UdpSocket, time::Duration};

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_DATAGRAM_SIZE: usize = 2048;

/// The line to the server: whole messages in, whole messages out.
pub trait Transport {
    /// Sends one message.
    fn send(&self, message: &str) -> io::Result<()>;

    /// Waits a short moment for a message and returns `None` if nothing arrived.
    fn receive(&self) -> io::Result<Option<String>>;
}

/// Talks to the server with UDP datagrams.
pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    /// Opens a socket towards the server at `host`:`port`.
    pub fn connect(host: &str, port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect((host, port))?;
        socket.set_read_timeout(Some(POLL_INTERVAL))?;
        Ok(Self { socket })
    }
}

impl Transport for UdpTransport {
    fn send(&self, message: &str) -> io::Result<()> {
        self.socket.send(message.as_bytes()).map(|_| ())
    }

    fn receive(&self) -> io::Result<Option<String>> {
        let mut buffer = [0; MAX_DATAGRAM_SIZE];
        let received = self.socket.recv(&mut buffer);
        message_from(received, &buffer)
    }
}

/// An interrupted or timed out read is not an error but just no message yet; the caller
/// polls again, which is also how Ctrl-C gets noticed.
fn message_from(received: io::Result<usize>, buffer: &[u8]) -> io::Result<Option<String>> {
    match received {
        Ok(size) => Ok(Some(String::from_utf8_lossy(&buffer[..size]).into_owned())),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::net::UdpSocket;

    use super::*;

    fn connected_pair() -> (UdpSocket, UdpTransport) {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        (server, UdpTransport::connect("127.0.0.1", port).unwrap())
    }

    #[test]
    fn sends_messages_to_the_server() {
        let (server, transport) = connected_pair();

        transport.send("REGISTER;anna").unwrap();

        let mut buffer = [0; 64];
        let (size, _) = server.recv_from(&mut buffer).unwrap();
        assert_eq!(&buffer[..size], b"REGISTER;anna");
    }

    #[test]
    fn receives_messages_from_the_server() {
        let (server, transport) = connected_pair();
        transport.send("hello").unwrap();
        let mut buffer = [0; 64];
        let (_, client) = server.recv_from(&mut buffer).unwrap();

        server.send_to(b"REGISTERED", client).unwrap();

        assert_eq!(transport.receive().unwrap().as_deref(), Some("REGISTERED"));
    }

    #[test]
    fn receives_nothing_when_the_server_is_silent() {
        let (_server, transport) = connected_pair();

        assert_eq!(transport.receive().unwrap(), None);
    }

    #[test]
    fn a_read_that_timed_out_or_was_interrupted_is_no_message() {
        for kind in [
            io::ErrorKind::WouldBlock,
            io::ErrorKind::TimedOut,
            io::ErrorKind::Interrupted,
        ] {
            assert_eq!(message_from(Err(kind.into()), &[]).unwrap(), None);
        }
    }

    #[test]
    fn other_read_errors_are_reported() {
        let error = message_from(Err(io::ErrorKind::ConnectionRefused.into()), &[]).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::ConnectionRefused);
    }

    #[test]
    fn invalid_utf8_is_replaced_instead_of_failing() {
        assert_eq!(
            message_from(Ok(2), &[0x68, 0xff]).unwrap().as_deref(),
            Some("h\u{fffd}")
        );
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use core::{
        cell::RefCell,
        sync::atomic::{AtomicBool, Ordering},
    };
    use std::{collections::VecDeque, thread, time::Duration};

    use super::{Transport, io};

    /// A scripted server: confirms (un)registration itself and then plays back its inbox.
    pub struct FakeTransport {
        inbox: RefCell<VecDeque<String>>,
        sent: RefCell<Vec<String>>,
        rejecting: bool,
        unresponsive: bool,
        broken: bool,
        ends_when_silent: bool,
        pub stop: AtomicBool,
    }

    impl FakeTransport {
        pub fn new(inbox: &[&str]) -> Self {
            Self {
                inbox: RefCell::new(inbox.iter().map(ToString::to_string).collect()),
                sent: RefCell::default(),
                rejecting: false,
                unresponsive: false,
                broken: false,
                ends_when_silent: true,
                stop: AtomicBool::new(false),
            }
        }

        pub fn rejecting(mut self) -> Self {
            self.rejecting = true;
            self
        }

        pub fn unresponsive(mut self) -> Self {
            self.unresponsive = true;
            self
        }

        pub fn broken(mut self) -> Self {
            self.broken = true;
            self
        }

        pub fn never_ending(mut self) -> Self {
            self.ends_when_silent = false;
            self
        }

        pub fn sent(&self) -> Vec<String> {
            self.sent.borrow().clone()
        }
    }

    impl Transport for FakeTransport {
        fn send(&self, message: &str) -> io::Result<()> {
            self.sent.borrow_mut().push(message.to_string());
            if self.unresponsive {
                return Ok(());
            }
            let answer = match message.split(';').next() {
                Some("REGISTER" | "REGISTER_SPECTATOR") if self.rejecting => {
                    "REJECTED;NAME_ALREADY_TAKEN"
                }
                Some("REGISTER" | "REGISTER_SPECTATOR") => "REGISTERED",
                Some("UNREGISTER") => "UNREGISTERED",
                _ => return Ok(()),
            };
            self.inbox.borrow_mut().push_front(answer.to_string());
            Ok(())
        }

        fn receive(&self) -> io::Result<Option<String>> {
            if self.broken {
                return Err(io::ErrorKind::ConnectionReset.into());
            }
            let next = self.inbox.borrow_mut().pop_front();
            if next.is_none() {
                if self.ends_when_silent {
                    self.stop.store(true, Ordering::Release);
                }
                thread::sleep(Duration::from_millis(1));
            }
            Ok(next)
        }
    }
}
