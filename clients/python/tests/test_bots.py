from collections.abc import Sequence

import pytest

from conftest import FakeTransport
from maexchen_client.bot import Bot, Role, Server
from maexchen_client.random_bot import RandomBot
from maexchen_client.simple_bot import SimpleBot
from maexchen_client.spectator_bot import SpectatorBot


def answers(bot: Bot, transport: FakeTransport, command: str, parameters: Sequence[str]) -> list[str]:
    bot.on_message(Server(transport), command, parameters)
    return transport.sent


def test_the_simple_bot_rolls_on_its_turn(transport: FakeTransport) -> None:
    assert answers(SimpleBot(), transport, "YOUR TURN", ["token-1"]) == ["ROLL;token-1"]


def test_the_simple_bot_announces_what_it_rolled(transport: FakeTransport) -> None:
    assert answers(SimpleBot(), transport, "ROLLED", ["3,1", "token-2"]) == ["ANNOUNCE;3,1;token-2"]


@pytest.mark.parametrize("bot", [SimpleBot(), RandomBot()])
def test_bots_ignore_everything_else(bot: Bot, transport: FakeTransport) -> None:
    assert answers(bot, transport, "SCORE", ["anna:1"]) == []
    assert answers(bot, transport, "YOUR TURN", []) == []


@pytest.mark.parametrize(("draw", "answer"), [(0.0, "ROLL;token-1"), (0.99, "SEE;token-1")])
def test_the_random_bot_rolls_or_looks_at_the_dice(
    monkeypatch: pytest.MonkeyPatch, transport: FakeTransport, draw: float, answer: str
) -> None:
    monkeypatch.setattr("random.random", lambda: draw)

    assert answers(RandomBot(), transport, "YOUR TURN", ["token-1"]) == [answer]


def test_the_random_bot_announces_what_it_rolled(transport: FakeTransport) -> None:
    assert answers(RandomBot(), transport, "ROLLED", ["5,2", "token-3"]) == ["ANNOUNCE;5,2;token-3"]


def test_the_spectator_bot_only_watches(transport: FakeTransport) -> None:
    bot = SpectatorBot()

    assert bot.role is Role.SPECTATOR
    assert answers(bot, transport, "YOUR TURN", ["token-1"]) == []
