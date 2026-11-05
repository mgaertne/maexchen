package maexchen;

import java.util.List;

/** Only watches the game. */
public final class SpectatorBot implements Bot {

  @Override
  public Role role() {
    return Role.SPECTATOR;
  }

  @Override
  public void onMessage(Server server, String command, List<String> parameters) {}
}
