from collections.abc import Sequence
from threading import Event

import pytest

from conftest import FakeTransport
from maexchen_client.bot import Bot, Role, Server
from maexchen_client.client import (
    HeartbeatMissingError,
    RegistrationRejectedError,
    RegistrationUnansweredError,
    Settings,
    run,
)
from maexchen_client.spectator_bot import SpectatorBot


class RecordingBot(Bot):
    def __init__(self) -> None:
        self.messages: list[tuple[str, list[str]]] = []

    def on_message(self, server: Server, command: str, parameters: Sequence[str]) -> None:
        self.messages.append((command, list(parameters)))


class StoppingTransport(FakeTransport):
    """Stops the client as soon as the prepared messages are used up."""

    def __init__(self, incoming: list[str], stop: Event) -> None:
        super().__init__(incoming)
        self.stop = stop

    def receive(self) -> str | None:
        if not self.incoming:
            self.stop.set()
            return "UNREGISTERED" if "UNREGISTER" in self.sent else None
        return super().receive()


def play(*messages: str, bot: Bot | None = None, settings: Settings | None = None) -> tuple[Bot, list[str]]:
    stop = Event()
    transport = StoppingTransport(["REGISTERED", *messages], stop)
    bot = bot or RecordingBot()
    run(transport, bot, settings or Settings(name="anna", answer_timeout=0.05), stop)
    return bot, transport.sent


def test_registers_a_player_and_unregisters_at_the_end() -> None:
    _, sent = play()
    assert sent == ["REGISTER;anna", "UNREGISTER"]


def test_registers_a_spectator_as_such() -> None:
    _, sent = play(bot=SpectatorBot())
    assert sent[0] == "REGISTER_SPECTATOR;anna"
    assert SpectatorBot.role is Role.SPECTATOR


def test_a_rejected_registration_is_an_error_and_needs_no_unregistration() -> None:
    transport = FakeTransport(["REJECTED;name taken"])
    with pytest.raises(RegistrationRejectedError):
        run(transport, RecordingBot(), Settings(name="anna", answer_timeout=0.05), Event())
    assert transport.sent == ["REGISTER;anna"]


def test_a_registration_without_an_answer_is_an_error() -> None:
    with pytest.raises(RegistrationUnansweredError):
        run(FakeTransport(), RecordingBot(), Settings(name="anna", answer_timeout=0.05), Event())


def test_hands_messages_without_heartbeats_to_the_bot() -> None:
    bot, _ = play("HEARTBEAT", "YOUR TURN;t")
    assert isinstance(bot, RecordingBot)
    assert bot.messages == [("YOUR TURN", ["t"])]


def test_joins_announced_rounds_by_itself() -> None:
    bot, sent = play("ROUND STARTING;t")
    assert isinstance(bot, RecordingBot)
    assert bot.messages == []
    assert "JOIN;t" in sent


def test_leaves_joining_to_the_bot_without_auto_join() -> None:
    bot, sent = play("ROUND STARTING;t", settings=Settings(name="anna", auto_join=False, answer_timeout=0.05))
    assert isinstance(bot, RecordingBot)
    assert bot.messages == [("ROUND STARTING", ["t"])]
    assert "JOIN;t" not in sent


def test_gives_up_when_the_heartbeats_stop() -> None:
    transport = FakeTransport(["REGISTERED"])
    settings = Settings(name="anna", heartbeat_timeout=0.05, answer_timeout=0.05)
    with pytest.raises(HeartbeatMissingError):
        run(transport, RecordingBot(), settings, Event())
    assert transport.sent[-1] == "UNREGISTER"
