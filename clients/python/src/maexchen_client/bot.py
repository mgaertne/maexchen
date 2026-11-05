"""The interface of a bot and the way it talks to the server."""

import logging
from collections.abc import Sequence
from enum import Enum
from typing import Protocol

from maexchen_client.transport import Transport

send_logger = logging.getLogger("send")


def readable(message: str) -> str:
    """Show a message the way it is logged: ``ROUND STARTING ['token']``."""
    command, *parameters = message.split(";")
    return f"{command} {parameters}"


class Role(Enum):
    """How a bot takes part in the game."""

    PLAYER = "REGISTER"
    SPECTATOR = "REGISTER_SPECTATOR"

    @property
    def register_command(self) -> str:
        """The command that registers a client in this role."""
        return self.value


class Server:
    """The server a bot talks to."""

    def __init__(self, transport: Transport) -> None:
        self._transport = transport

    def send(self, command: str, *parameters: str) -> None:
        """Send ``command`` with its parameters, e.g. ``send("ANNOUNCE", "6,5", token)``.

        The parts are joined with ``;``. A failing send is logged and otherwise ignored, as UDP
        gives no guarantee anyway.
        """
        message = ";".join((command, *parameters))
        send_logger.info("----> %s", readable(message))
        try:
            self._transport.send(message)
        except OSError as error:
            send_logger.warning("Failed to send %s: %s", readable(message), error)


class Bot(Protocol):
    """The behaviour of a bot; write your own by subclassing this and overriding ``on_message``.

    The client registers the bot, joins rounds, watches heartbeats and unregisters it again.
    What is left for the bot is to react to the messages of the game.

    Messages from the server (``command``: ``parameters``):

    * ``ROUND STARTING``: ``[token]``, an invitation; answer with ``JOIN`` unless the client joins
      by itself
    * ``ROUND STARTED``: ``[round, players]``, the players in turn order separated by ``,``
    * ``ROUND CANCELED``: ``[reason]``
    * ``YOUR TURN``: ``[token]``; answer with ``ROLL`` or ``SEE``
    * ``ROLLED``: ``[dice, token]``, e.g. ``["3,1", token]``; answer with ``ANNOUNCE``
    * ``PLAYER ROLLS``: ``[player]``
    * ``PLAYER WANTS TO SEE``: ``[player]``
    * ``ANNOUNCED``: ``[player, dice]``
    * ``ACTUAL DICE``: ``[dice]``, shown when somebody looked at the dice
    * ``PLAYER LOST``: ``[players, reason]``
    * ``SCORE``: ``[scores]``, as ``player:points`` pairs separated by ``,``

    ``ROLL``, ``SEE`` and ``JOIN`` take the token of the message they answer. ``ANNOUNCE`` takes
    the dice and the token of the ``ROLLED`` message.
    """

    role: Role = Role.PLAYER

    def on_message(self, server: Server, command: str, parameters: Sequence[str]) -> None:
        """Handle every message of the server except heartbeats.

        ``YOUR TURN;4711`` arrives as ``"YOUR TURN"`` and ``["4711"]``. Answer with
        ``server.send``. Invitations arrive only without auto join (``--no-auto-join``).
        """
        ...
