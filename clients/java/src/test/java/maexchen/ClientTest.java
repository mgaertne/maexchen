package maexchen;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicBoolean;
import org.junit.jupiter.api.Test;

class ClientTest {

  private static final Settings SETTINGS =
      new Settings("anna", true, Optional.of(Duration.ofSeconds(5)), Duration.ofMillis(50));

  private static final class RecordingBot implements Bot {
    final List<String> messages = new ArrayList<>();

    @Override
    public void onMessage(Server server, String command, List<String> parameters) {
      messages.add(command + parameters);
    }
  }

  private static FakeTransport playedThrough(AtomicBoolean stop, String... messages) {
    var all = new ArrayList<>(List.of("REGISTERED"));
    all.addAll(List.of(messages));
    return new FakeTransport(() -> stop.set(true), all.toArray(String[]::new));
  }

  @Test
  void registersAPlayerAndUnregistersAtTheEnd() {
    var stop = new AtomicBoolean();
    var transport = playedThrough(stop);

    Client.run(transport, new RecordingBot(), SETTINGS, stop::get);

    assertEquals(List.of("REGISTER;anna", "UNREGISTER"), transport.sent);
  }

  @Test
  void registersASpectatorAsSuch() {
    var stop = new AtomicBoolean();
    var transport = playedThrough(stop);

    Client.run(transport, new SpectatorBot(), SETTINGS, stop::get);

    assertEquals("REGISTER_SPECTATOR;anna", transport.sent.getFirst());
  }

  @Test
  void aRejectedRegistrationIsAnErrorAndNeedsNoUnregistration() {
    var transport = new FakeTransport("REJECTED;name taken");

    assertThrows(
        ClientException.RegistrationRejected.class,
        () -> Client.run(transport, new RecordingBot(), SETTINGS, () -> false));

    assertEquals(List.of("REGISTER;anna"), transport.sent);
  }

  @Test
  void aRegistrationWithoutAnAnswerIsAnError() {
    assertThrows(
        ClientException.RegistrationUnanswered.class,
        () -> Client.run(new FakeTransport(), new RecordingBot(), SETTINGS, () -> false));
  }

  @Test
  void handsMessagesWithoutHeartbeatsToTheBot() {
    var stop = new AtomicBoolean();
    var bot = new RecordingBot();

    Client.run(playedThrough(stop, "HEARTBEAT", "YOUR TURN;t"), bot, SETTINGS, stop::get);

    assertEquals(List.of("YOUR TURN[t]"), bot.messages);
  }

  @Test
  void joinsAnnouncedRoundsByItself() {
    var stop = new AtomicBoolean();
    var bot = new RecordingBot();
    var transport = playedThrough(stop, "ROUND STARTING;t");

    Client.run(transport, bot, SETTINGS, stop::get);

    assertTrue(bot.messages.isEmpty());
    assertTrue(transport.sent.contains("JOIN;t"));
  }

  @Test
  void leavesJoiningToTheBotWithoutAutoJoin() {
    var stop = new AtomicBoolean();
    var bot = new RecordingBot();
    var transport = playedThrough(stop, "ROUND STARTING;t");
    var settings =
        new Settings("anna", false, SETTINGS.heartbeatTimeout(), SETTINGS.answerTimeout());

    Client.run(transport, bot, settings, stop::get);

    assertEquals(List.of("ROUND STARTING[t]"), bot.messages);
    assertFalse(transport.sent.contains("JOIN;t"));
  }

  @Test
  void givesUpWhenTheHeartbeatsStop() {
    var transport = new FakeTransport("REGISTERED");
    var settings =
        new Settings("anna", true, Optional.of(Duration.ofMillis(50)), Duration.ofMillis(50));

    assertThrows(
        ClientException.HeartbeatMissing.class,
        () -> Client.run(transport, new RecordingBot(), settings, () -> false));

    assertEquals("UNREGISTER", transport.sent.getLast());
  }
}
