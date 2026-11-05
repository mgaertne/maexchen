package maexchen;

import java.util.List;

/**
 * The behaviour of a bot; write your own by implementing this interface.
 *
 * <p>The client registers the bot, joins rounds, watches heartbeats and unregisters it again. What
 * is left for the bot is to react to the messages of the game.
 *
 * <p>Messages from the server ({@code command}: {@code parameters}):
 *
 * <ul>
 *   <li>{@code ROUND STARTING}: {@code [token]}, an invitation; answer with {@code JOIN} unless the
 *       client joins by itself
 *   <li>{@code ROUND STARTED}: {@code [round, players]}, the players in turn order separated by
 *       {@code ,}
 *   <li>{@code ROUND CANCELED}: {@code [reason]}
 *   <li>{@code YOUR TURN}: {@code [token]}; answer with {@code ROLL} or {@code SEE}
 *   <li>{@code ROLLED}: {@code [dice, token]}, for example {@code ["3,1", token]}; answer with
 *       {@code ANNOUNCE}
 *   <li>{@code PLAYER ROLLS}: {@code [player]}
 *   <li>{@code PLAYER WANTS TO SEE}: {@code [player]}
 *   <li>{@code ANNOUNCED}: {@code [player, dice]}
 *   <li>{@code ACTUAL DICE}: {@code [dice]}, shown when somebody looked at the dice
 *   <li>{@code PLAYER LOST}: {@code [players, reason]}
 *   <li>{@code SCORE}: {@code [scores]}, as {@code player:points} pairs separated by {@code ,}
 * </ul>
 *
 * <p>{@code ROLL}, {@code SEE} and {@code JOIN} take the token of the message they answer. {@code
 * ANNOUNCE} takes the dice and the token of the {@code ROLLED} message.
 */
public interface Bot {

  /** Whether the bot plays or only watches; spectators register differently. */
  default Role role() {
    return Role.PLAYER;
  }

  /**
   * Called for every message of the server except heartbeats, and for invitations only when the
   * client does not join by itself ({@code --no-auto-join}).
   *
   * <p>{@code YOUR TURN;4711} arrives as {@code "YOUR TURN"} and {@code ["4711"]}. Answer with
   * {@link Server#send}.
   */
  void onMessage(Server server, String command, List<String> parameters);
}
