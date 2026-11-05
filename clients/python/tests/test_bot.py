from maexchen_client.bot import Role, readable


def test_messages_are_logged_with_quoted_parameters_in_brackets() -> None:
    assert readable("ROUND STARTING;af99f6db") == "ROUND STARTING ['af99f6db']"
    assert readable("ROUND STARTED;514105;anna,bert") == "ROUND STARTED ['514105', 'anna,bert']"
    assert readable("UNREGISTER") == "UNREGISTER []"


def test_every_role_registers_with_its_own_command() -> None:
    assert Role.PLAYER.register_command == "REGISTER"
    assert Role.SPECTATOR.register_command == "REGISTER_SPECTATOR"
