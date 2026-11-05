import pytest

from maexchen_client.main import parse_arguments, settings_from


def test_joins_and_watches_heartbeats_by_default() -> None:
    arguments = parse_arguments([])
    assert arguments.auto_join
    assert arguments.watch_heartbeat


def test_the_negated_flags_turn_the_behaviour_off() -> None:
    settings = settings_from(parse_arguments(["--no-auto-join", "--no-watch-heartbeat"]))
    assert not settings.auto_join
    assert settings.heartbeat_timeout is None


def test_invents_a_name_unless_one_is_given() -> None:
    assert settings_from(parse_arguments([])).name.startswith("maexchen-python-bot-")
    assert settings_from(parse_arguments(["-n", "anna"])).name == "anna"


def test_rejects_unknown_bot_types() -> None:
    with pytest.raises(SystemExit):
        parse_arguments(["-t", "nonsense"])
