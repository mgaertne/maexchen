package maexchen;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class MainTest {

  @Test
  void joinsAndWatchesHeartbeatsByDefault() {
    var options = Main.Options.parse();

    assertTrue(options.autoJoin());
    assertTrue(options.watchHeartbeat());
    assertEquals(9000, options.port());
    assertTrue(options.name().startsWith("maexchen-java-bot-"));
  }

  @Test
  void theNegatedFlagsTurnTheBehaviourOff() {
    var options = Main.Options.parse("--no-auto-join", "--no-watch-heartbeat");

    assertFalse(options.autoJoin());
    assertTrue(options.settings().heartbeatTimeout().isEmpty());
  }

  @Test
  void takesAddressNameAndType() {
    var options = Main.Options.parse("-i", "10.0.0.1", "-p", "4000", "-n", "anna", "-t", "random");

    assertEquals("10.0.0.1", options.ip());
    assertEquals(4000, options.port());
    assertEquals("anna", options.name());
    assertEquals("random", options.type());
  }

  @Test
  void rejectsUnknownBotTypesAndOptions() {
    assertThrows(IllegalArgumentException.class, () -> Main.Options.parse("-t", "nonsense"));
    assertThrows(IllegalArgumentException.class, () -> Main.Options.parse("--nonsense"));
    assertThrows(IllegalArgumentException.class, () -> Main.Options.parse("-n"));
  }
}
