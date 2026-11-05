package maexchen;

import java.io.IOException;
import java.util.Optional;

/** The line to the server: whole messages in, whole messages out. */
public interface Transport {

  /** Sends one message. */
  void send(String message) throws IOException;

  /** Waits a short moment for a message and returns empty if nothing arrived. */
  Optional<String> receive() throws IOException;
}
