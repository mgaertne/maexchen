from collections import deque
from collections.abc import Iterable

import pytest


class FakeTransport:
    """Hands out prepared messages, then silence, and records what was sent."""

    def __init__(self, incoming: Iterable[str] = ()) -> None:
        self.incoming: deque[str] = deque(incoming)
        self.sent: list[str] = []
        self.on_silence: list[str] = []

    def send(self, message: str) -> None:
        self.sent.append(message)

    def receive(self) -> str | None:
        return self.incoming.popleft() if self.incoming else None


@pytest.fixture
def transport() -> FakeTransport:
    return FakeTransport()
