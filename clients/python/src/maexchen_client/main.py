"""Command line front end of the Maexchen client; the framework lives in the other modules."""

import argparse
import logging
import random
import signal
import sys
from collections.abc import Callable, Sequence
from threading import Event

from maexchen_client.bot import Bot
from maexchen_client.client import ClientError, Settings, run
from maexchen_client.random_bot import RandomBot
from maexchen_client.simple_bot import SimpleBot
from maexchen_client.spectator_bot import SpectatorBot
from maexchen_client.transport import UdpTransport

BOT_TYPES: dict[str, Callable[[], Bot]] = {
    "simple": SimpleBot,
    "random": RandomBot,
    "spectator": SpectatorBot,
}


def random_bot_name() -> str:
    """Invent a name for a bot that was not given one."""
    return f"maexchen-python-bot-{random.randint(100, 999)}"


def parse_arguments(arguments: Sequence[str] | None = None) -> argparse.Namespace:
    """Read the command line."""
    parser = argparse.ArgumentParser(description="MaexchenBot")
    parser.add_argument("--ip", "-i", default="127.0.0.1", help="the server ip to connect to (default: %(default)s)")
    parser.add_argument("--port", "-p", type=int, default=9000, help="the server port (default: %(default)s)")
    parser.add_argument("--name", "-n", help="the player's name to use (default: maexchen-python-bot-NNN)")
    parser.add_argument(
        "--auto-join",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="automatically join new announced matches (default: %(default)s)",
    )
    parser.add_argument(
        "--watch-heartbeat",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="give up when the server sends no heartbeats any more (default: %(default)s)",
    )
    parser.add_argument(
        "--type", "-t", choices=BOT_TYPES, default="simple", help="the flavor of bot to use (default: %(default)s)"
    )
    parser.add_argument("--verbose", "-v", action="store_true", help="also log heartbeats")
    parser.add_argument("--quiet", "-q", action="store_true", help="log warnings and errors only")
    return parser.parse_args(arguments)


def settings_from(arguments: argparse.Namespace) -> Settings:
    """Turn the command line into the settings of the client."""
    defaults = Settings(name="")
    return Settings(
        name=arguments.name or random_bot_name(),
        auto_join=arguments.auto_join,
        heartbeat_timeout=defaults.heartbeat_timeout if arguments.watch_heartbeat else None,
    )


def configure_logging(arguments: argparse.Namespace) -> None:
    """Log to the console; more with --verbose, less with --quiet."""
    level = logging.DEBUG if arguments.verbose else logging.WARNING if arguments.quiet else logging.INFO
    logging.basicConfig(level=level, format="%(asctime)s | %(levelname)-8s | %(name)-20s | %(message)s")


def main() -> None:
    """Run the client until Ctrl-C or until the server is gone."""
    arguments = parse_arguments()
    configure_logging(arguments)

    stop = Event()
    signal.signal(signal.SIGINT, lambda _signal, _frame: stop.set())

    with UdpTransport(arguments.ip, arguments.port) as transport:
        try:
            run(transport, BOT_TYPES[arguments.type](), settings_from(arguments), stop)
        except ClientError as error:
            logging.getLogger("client").error("%s", error)  # noqa: TRY400
            sys.exit(1)


if __name__ == "__main__":
    main()
