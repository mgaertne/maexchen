package maexchen;

import java.io.IOException;
import java.lang.System.Logger;
import java.lang.System.Logger.Level;
import java.util.stream.Stream;

/** The server a bot talks to. */
public final class Server {

  private static final Logger LOG = System.getLogger("send");

  private final Transport transport;

  public Server(Transport transport) {
    this.transport = transport;
  }

  /**
   * Sends {@code command} with its parameters, for example {@code send("ANNOUNCE", "6,5", token)}.
   *
   * <p>The parts are joined with {@code ;}. A failing send is logged and otherwise ignored, as UDP
   * gives no guarantee anyway.
   */
  public void send(String command, String... parameters) {
    String message =
        String.join(";", Stream.concat(Stream.of(command), Stream.of(parameters)).toList());
    LOG.log(Level.INFO, "----> {0}", readable(message));
    try {
      transport.send(message);
    } catch (IOException error) {
      LOG.log(Level.WARNING, "Failed to send {0}: {1}", readable(message), error.getMessage());
    }
  }

  /** Shows a message the way it is logged: {@code ROUND STARTING ['token']}. */
  static String readable(String message) {
    String[] parts = message.split(";");
    var parameters = Stream.of(parts).skip(1).map(parameter -> "'" + parameter + "'").toList();
    return parts[0] + " " + parameters;
  }
}
