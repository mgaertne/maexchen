package maexchen;

import java.io.IOException;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.Supplier;
import java.util.random.RandomGenerator;

/** Command line front end of the Maexchen client; the framework lives in the other classes. */
public final class Main {

  private Main() {}

  private static final Map<String, Supplier<Bot>> BOT_TYPES =
      Map.of("simple", SimpleBot::new, "random", RandomBot::new, "spectator", SpectatorBot::new);

  private static final String USAGE =
      """
      usage: gradle run --args "[-i IP] [-p PORT] [-n NAME] [-t simple|random|spectator]
                                [--no-auto-join] [--no-watch-heartbeat]"
      """;

  /** The options of the command line. */
  record Options(
      String ip, int port, String name, String type, boolean autoJoin, boolean watchHeartbeat) {

    static Options parse(String... arguments) {
      String ip = "127.0.0.1";
      int port = 9000;
      String name = "maexchen-java-bot-" + RandomGenerator.getDefault().nextInt(100, 1000);
      String type = "simple";
      boolean autoJoin = true;
      boolean watchHeartbeat = true;
      for (int index = 0; index < arguments.length; index++) {
        switch (arguments[index]) {
          case "-i", "--ip" -> ip = argument(arguments, ++index);
          case "-p", "--port" -> port = Integer.parseInt(argument(arguments, ++index));
          case "-n", "--name" -> name = argument(arguments, ++index);
          case "-t", "--type" -> type = argument(arguments, ++index);
          case "--no-auto-join" -> autoJoin = false;
          case "--no-watch-heartbeat" -> watchHeartbeat = false;
          default -> throw new IllegalArgumentException("unknown option " + arguments[index]);
        }
      }
      if (!BOT_TYPES.containsKey(type)) {
        throw new IllegalArgumentException("unknown bot type " + type);
      }
      return new Options(ip, port, name, type, autoJoin, watchHeartbeat);
    }

    private static String argument(String[] arguments, int index) {
      if (index >= arguments.length) {
        throw new IllegalArgumentException("missing value for " + arguments[index - 1]);
      }
      return arguments[index];
    }

    Settings settings() {
      return new Settings(
          name,
          autoJoin,
          watchHeartbeat ? Optional.of(Settings.HEARTBEAT_TIMEOUT) : Optional.empty(),
          Settings.ANSWER_TIMEOUT);
    }
  }

  public static void main(String[] arguments) throws IOException {
    System.setProperty(
        "java.util.logging.SimpleFormatter.format",
        "%1$tF %1$tT,%1$tL | %4$-8s | %3$-20s | %5$s%6$s%n");
    Options options;
    try {
      options = Options.parse(arguments);
    } catch (IllegalArgumentException error) {
      System.err.println(error.getMessage());
      System.err.print(USAGE);
      System.exit(2);
      return;
    }

    var stop = new AtomicBoolean();
    var finished = new CountDownLatch(1);
    Runtime.getRuntime()
        .addShutdownHook(
            new Thread(() -> {
              stop.set(true);
              try {
                finished.await(Settings.ANSWER_TIMEOUT.toMillis() * 2, TimeUnit.MILLISECONDS);
              } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
              }
            }));

    int status = 0;
    try (var transport = new UdpTransport(options.ip(), options.port())) {
      Client.run(transport, BOT_TYPES.get(options.type()).get(), options.settings(), stop::get);
    } catch (ClientException error) {
      System.getLogger("client").log(System.Logger.Level.ERROR, error.getMessage());
      status = 1;
    } finally {
      finished.countDown();
    }
    System.exit(status);
  }
}
