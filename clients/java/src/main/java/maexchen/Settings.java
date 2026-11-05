package maexchen;

import java.time.Duration;
import java.util.Optional;

/**
 * How the client takes part in the game, independent of the bot.
 *
 * @param name the name to register with
 * @param autoJoin whether the client answers invitations by itself instead of passing them on
 * @param heartbeatTimeout how long the server may stay silent before the client gives up; empty
 *     never gives up
 * @param answerTimeout how long the client waits for the answer to a registration
 */
public record Settings(
    String name, boolean autoJoin, Optional<Duration> heartbeatTimeout, Duration answerTimeout) {

  public static final Duration HEARTBEAT_TIMEOUT = Duration.ofSeconds(5);
  public static final Duration ANSWER_TIMEOUT = Duration.ofSeconds(2);

  /** Settings with automatic joining and the usual timeouts. */
  public static Settings of(String name) {
    return new Settings(name, true, Optional.of(HEARTBEAT_TIMEOUT), ANSWER_TIMEOUT);
  }
}
