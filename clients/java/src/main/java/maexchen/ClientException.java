package maexchen;

import java.time.Duration;

/** Why a client stopped with an error. */
public class ClientException extends RuntimeException {

  public ClientException(String message) {
    super(message);
  }

  public ClientException(String message, Throwable cause) {
    super(message, cause);
  }

  /** The server refused the registration. */
  public static final class RegistrationRejected extends ClientException {
    public RegistrationRejected(String message) {
      super("the server rejected the registration: " + message);
    }
  }

  /** The server did not answer the registration in time. */
  public static final class RegistrationUnanswered extends ClientException {
    public RegistrationUnanswered() {
      super("the server did not answer the registration");
    }
  }

  /** The server stayed silent for longer than the allowed time. */
  public static final class HeartbeatMissing extends ClientException {
    public HeartbeatMissing(Duration limit) {
      super("the server sent no heartbeat for " + limit);
    }
  }

  /** Sending to or receiving from the server failed. */
  public static final class ConnectionLost extends ClientException {
    public ConnectionLost(Throwable cause) {
      super("lost the connection to the server", cause);
    }
  }
}
