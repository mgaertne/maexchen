"""Watches the game without taking part."""

from collections.abc import Sequence

from maexchen_client.bot import Bot, Role, Server


class SpectatorBot(Bot):
    """Only watches the game."""

    role = Role.SPECTATOR

    def on_message(self, server: Server, command: str, parameters: Sequence[str]) -> None:
        pass
