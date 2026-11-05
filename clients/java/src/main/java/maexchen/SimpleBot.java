package maexchen;

import java.util.List;

/** Rolls on its turn and announces exactly what it rolled. */
public final class SimpleBot implements Bot {

  @Override
  public void onMessage(Server server, String command, List<String> parameters) {
    switch (command) {
      case "YOUR TURN" -> server.send("ROLL", parameters.getLast());
      case "ROLLED" -> server.send("ANNOUNCE", parameters.getFirst(), parameters.getLast());
      default -> {}
    }
  }
}
