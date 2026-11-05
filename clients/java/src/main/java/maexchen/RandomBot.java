package maexchen;

import java.util.List;
import java.util.random.RandomGenerator;

/** Rolls or looks at the dice at random on its turn and announces what it rolled. */
public final class RandomBot implements Bot {

  private static final double CHANCE_TO_ROLL = 0.8;

  private final RandomGenerator random;

  public RandomBot() {
    this(RandomGenerator.getDefault());
  }

  RandomBot(RandomGenerator random) {
    this.random = random;
  }

  @Override
  public void onMessage(Server server, String command, List<String> parameters) {
    switch (command) {
      case "YOUR TURN" ->
          server.send(random.nextDouble() < CHANCE_TO_ROLL ? "ROLL" : "SEE", parameters.getLast());
      case "ROLLED" -> server.send("ANNOUNCE", parameters.getFirst(), parameters.getLast());
      default -> {}
    }
  }
}
