"""Rolls or looks at the dice at random."""

import random
from collections.abc import Sequence

from maexchen_client.bot import Bot, Server

CHANCE_TO_ROLL = 0.8


class RandomBot(Bot):
    """Rolls or looks at the dice at random on its turn and announces what it rolled."""

    def on_message(self, server: Server, command: str, parameters: Sequence[str]) -> None:
        match command, parameters:
            case "YOUR TURN", [*_, token]:
                server.send("ROLL" if random.random() < CHANCE_TO_ROLL else "SEE", token)
            case "ROLLED", [dice, *_, token]:
                server.send("ANNOUNCE", dice, token)
