package maexchen;

import java.io.IOException;
import java.lang.System.Logger;
import java.lang.System.Logger.Level;
import java.time.Duration;
import java.time.Instant;
import java.util.List;
import java.util.Optional;
import java.util.function.BooleanSupplier;

/** The generic part of every bot: register, pass the messages on, unregister. */
public final class Client {

  private static final Logger RECEIVE = System.getLogger("receive");
  private static final Logger CLIENT = System.getLogger("client");

  private final Transport transport;
  private final Server server;
  private final Bot bot;
  private final Settings settings;

  private Client(Transport transport, Bot bot, Settings settings) {
    this.transport = transport;
    this.server = new Server(transport);
    this.bot = bot;
    this.settings = settings;
  }

  /**
   * Registers the bot, hands it all messages until {@code stop} reports true, and unregisters it
   * again.
   *
   * @throws ClientException if the registration fails, the server falls silent or the connection
   *     breaks; a registered bot is unregistered in every case
   */
  public static void run(Transport transport, Bot bot, Settings settings, BooleanSupplier stop) {
    try {
      new Client(transport, bot, settings).run(stop);
    } catch (IOException error) {
      throw new ClientException.ConnectionLost(error);
    }
  }

  private void run(BooleanSupplier stop) throws IOException {
    register();
    try {
      play(stop);
    } finally {
      unregister();
    }
  }

  private void register() throws IOException {
    server.send(bot.role().registerCommand(), settings.name());
    String answer =
        awaitAnswer("REGISTERED").orElseThrow(ClientException.RegistrationUnanswered::new);
    if (!commandOf(answer).equals("REGISTERED")) {
      throw new ClientException.RegistrationRejected(answer);
    }
  }

  private void play(BooleanSupplier stop) throws IOException {
    Instant lastHeartbeat = Instant.now();
    while (!stop.getAsBoolean()) {
      Optional<Duration> limit = settings.heartbeatTimeout();
      if (limit.isPresent()
          && Duration.between(lastHeartbeat, Instant.now()).compareTo(limit.get()) > 0) {
        throw new ClientException.HeartbeatMissing(limit.get());
      }

      Optional<String> received = transport.receive();
      if (received.isEmpty()) {
        continue;
      }
      String message = received.get();
      String command = commandOf(message);
      List<String> parameters = parametersOf(message);

      if (command.equals("HEARTBEAT")) {
        RECEIVE.log(Level.DEBUG, "<---- {0}", Server.readable(message));
        lastHeartbeat = Instant.now();
      } else if (command.equals("ROUND STARTING") && settings.autoJoin() && !parameters.isEmpty()) {
        RECEIVE.log(Level.INFO, "<---- {0}", Server.readable(message));
        server.send("JOIN", parameters.getLast());
      } else {
        RECEIVE.log(Level.INFO, "<---- {0}", Server.readable(message));
        bot.onMessage(server, command, parameters);
      }
    }
  }

  private void unregister() throws IOException {
    server.send("UNREGISTER");
    if (awaitAnswer("UNREGISTERED").isEmpty()) {
      CLIENT.log(Level.WARNING, "The server did not confirm the unregistration.");
    }
  }

  private Optional<String> awaitAnswer(String confirmation) throws IOException {
    Instant deadline = Instant.now().plus(settings.answerTimeout());
    while (Instant.now().isBefore(deadline)) {
      Optional<String> received = transport.receive();
      if (received.isEmpty()) {
        continue;
      }
      RECEIVE.log(Level.INFO, "<---- {0}", Server.readable(received.get()));
      String command = commandOf(received.get());
      if (command.equals(confirmation) || command.equals("REJECTED")) {
        return received;
      }
    }
    return Optional.empty();
  }

  private static String commandOf(String message) {
    return message.split(";", -1)[0];
  }

  private static List<String> parametersOf(String message) {
    String[] parts = message.split(";", -1);
    return List.of(parts).subList(1, parts.length);
  }
}
