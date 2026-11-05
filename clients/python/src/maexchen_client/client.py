"""The generic part of every bot: register, pass the messages on, unregister."""

import logging
import time
from dataclasses import dataclass
from threading import Event

from maexchen_client.bot import Bot, Server, readable
from maexchen_client.transport import Transport

HEARTBEAT_TIMEOUT = 5.0
ANSWER_TIMEOUT = 2.0

receive_logger = logging.getLogger("receive")
client_logger = logging.getLogger("client")


class ClientError(Exception):
    """Why a client stopped with an error."""


class RegistrationRejectedError(ClientError):
    """The server refused the registration; carries its message."""


class RegistrationUnansweredError(ClientError):
    """The server did not answer the registration in time."""


class HeartbeatMissingError(ClientError):
    """The server stayed silent for longer than the allowed time."""


class ConnectionLostError(ClientError):
    """Sending to or receiving from the server failed."""


@dataclass(frozen=True, slots=True)
class Settings:
    """How the client takes part in the game, independent of the bot."""

    name: str
    auto_join: bool = True
    heartbeat_timeout: float | None = HEARTBEAT_TIMEOUT
    answer_timeout: float = ANSWER_TIMEOUT


def run(transport: Transport, bot: Bot, settings: Settings, stop: Event) -> None:
    """Register the bot, hand it all messages until ``stop`` is set, and unregister it again.

    Raises a ``ClientError`` if the registration fails, the server falls silent or the connection
    breaks. A registered bot is unregistered in every case.
    """
    server = Server(transport)
    try:
        _register(transport, server, bot, settings)
        try:
            _play(transport, server, bot, settings, stop)
        finally:
            _unregister(transport, server, settings.answer_timeout)
    except OSError as error:
        raise ConnectionLostError(str(error)) from error


def _register(transport: Transport, server: Server, bot: Bot, settings: Settings) -> None:
    server.send(bot.role.register_command, settings.name)
    answer = _await_answer(transport, "REGISTERED", settings.answer_timeout)
    if answer is None:
        raise RegistrationUnansweredError("the server did not answer the registration")
    if answer.split(";")[0] != "REGISTERED":
        raise RegistrationRejectedError(f"the server rejected the registration: {answer}")


def _play(transport: Transport, server: Server, bot: Bot, settings: Settings, stop: Event) -> None:
    last_heartbeat = time.monotonic()
    while not stop.is_set():
        limit = settings.heartbeat_timeout
        if limit is not None and time.monotonic() - last_heartbeat > limit:
            raise HeartbeatMissingError(f"the server sent no heartbeat for {limit} seconds")

        message = transport.receive()
        if message is None:
            continue
        command, *parameters = message.split(";")

        if command == "HEARTBEAT":
            receive_logger.debug("<---- %s", readable(message))
            last_heartbeat = time.monotonic()
            continue

        receive_logger.info("<---- %s", readable(message))
        if command == "ROUND STARTING" and settings.auto_join and parameters:
            server.send("JOIN", parameters[-1])
        else:
            bot.on_message(server, command, parameters)


def _unregister(transport: Transport, server: Server, timeout: float) -> None:
    server.send("UNREGISTER")
    if _await_answer(transport, "UNREGISTERED", timeout) is None:
        client_logger.warning("The server did not confirm the unregistration.")


def _await_answer(transport: Transport, confirmation: str, timeout: float) -> str | None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        message = transport.receive()
        if message is None:
            continue
        receive_logger.info("<---- %s", readable(message))
        if message.split(";")[0] in (confirmation, "REJECTED"):
            return message
    return None
