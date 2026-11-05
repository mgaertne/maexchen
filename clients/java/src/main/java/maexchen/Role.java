package maexchen;

/** How a bot takes part in the game. */
public enum Role {
  /** Takes part in rounds. */
  PLAYER("REGISTER"),
  /** Only watches. */
  SPECTATOR("REGISTER_SPECTATOR");

  private final String registerCommand;

  Role(String registerCommand) {
    this.registerCommand = registerCommand;
  }

  /** The command that registers a client in this role. */
  public String registerCommand() {
    return registerCommand;
  }
}
