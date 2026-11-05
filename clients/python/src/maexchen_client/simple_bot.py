"""Rolls on its turn and announces exactly what it rolled."""

from collections.abc import Sequence

from maexchen_client.bot import Bot, Server


class SimpleBot(Bot):
    """Rolls on its turn and announces exactly what it rolled."""

    def on_message(self, server: Server, command: str, parameters: Sequence[str]) -> None:
        match command, parameters:
            case "YOUR TURN", [*_, token]:
                server.send("ROLL", token)
            case "ROLLED", [dice, *_, token]:
                server.send("ANNOUNCE", dice, token)
