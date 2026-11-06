package maexchen;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import java.util.random.RandomGenerator;
import org.junit.jupiter.api.Test;

class BotsTest {

  private static List<String> answers(Bot bot, String command, String... parameters) {
    var transport = new FakeTransport();
    bot.onMessage(new Server(transport), command, List.of(parameters));
    return transport.sent;
  }

  private static RandomBot randomBotDrawing(long bits) {
    return new RandomBot(() -> bits);
  }

  @Test
  void theSimpleBotRollsOnItsTurn() {
    assertEquals(List.of("ROLL;token-1"), answers(new SimpleBot(), "YOUR TURN", "token-1"));
  }

  @Test
  void theSimpleBotAnnouncesWhatItRolled() {
    assertEquals(
        List.of("ANNOUNCE;3,1;token-2"), answers(new SimpleBot(), "ROLLED", "3,1", "token-2"));
  }

  @Test
  void theSimpleBotIgnoresEverythingElse() {
    assertTrue(answers(new SimpleBot(), "SCORE", "anna:1").isEmpty());
  }

  @Test
  void theRandomBotRollsWhenTheDrawIsLow() {
    assertEquals(List.of("ROLL;token-1"), answers(randomBotDrawing(0L), "YOUR TURN", "token-1"));
  }

  @Test
  void theRandomBotLooksAtTheDiceWhenTheDrawIsHigh() {
    assertEquals(List.of("SEE;token-1"), answers(randomBotDrawing(-1L), "YOUR TURN", "token-1"));
  }

  @Test
  void theRandomBotAnnouncesWhatItRolled() {
    assertEquals(
        List.of("ANNOUNCE;5,2;token-3"), answers(new RandomBot(), "ROLLED", "5,2", "token-3"));
  }

  @Test
  void theRandomBotIgnoresEverythingElse() {
    assertTrue(answers(new RandomBot(RandomGenerator.getDefault()), "SCORE", "anna:1").isEmpty());
  }

  @Test
  void theSpectatorBotOnlyWatches() {
    var bot = new SpectatorBot();

    assertEquals(Role.SPECTATOR, bot.role());
    assertTrue(answers(bot, "YOUR TURN", "token-1").isEmpty());
  }
}
