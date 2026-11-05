"""How messages get to and from the server."""

import socket
from types import TracebackType
from typing import Protocol, Self

POLL_INTERVAL = 0.25
MAX_DATAGRAM_SIZE = 2048


class Transport(Protocol):
    """The line to the server: whole messages in, whole messages out."""

    def send(self, message: str) -> None:
        """Send one message."""
        ...

    def receive(self) -> str | None:
        """Wait a short moment for a message; ``None`` if nothing arrived."""
        ...


class UdpTransport:
    """Talk to the server with UDP datagrams."""

    def __init__(self, host: str, port: int) -> None:
        self._socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self._socket.settimeout(POLL_INTERVAL)
        self._socket.connect((host, port))

    def send(self, message: str) -> None:
        self._socket.send(message.encode("utf-8"))

    def receive(self) -> str | None:
        try:
            return self._socket.recv(MAX_DATAGRAM_SIZE).decode("utf-8", errors="replace")
        except (TimeoutError, InterruptedError):
            return None

    def close(self) -> None:
        self._socket.close()

    def __enter__(self) -> Self:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        self.close()
