package maexchen;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.Queue;

/** Hands out prepared messages, then silence, and records what was sent. */
final class FakeTransport implements Transport {

  final List<String> sent = new ArrayList<>();
  private final Queue<String> incoming = new ArrayDeque<>();
  private final Runnable onSilence;

  FakeTransport(String... incoming) {
    this(() -> {}, incoming);
  }

  FakeTransport(Runnable onSilence, String... incoming) {
    this.incoming.addAll(List.of(incoming));
    this.onSilence = onSilence;
  }

  @Override
  public void send(String message) {
    sent.add(message);
  }

  @Override
  public Optional<String> receive() {
    String next = incoming.poll();
    if (next == null) {
      onSilence.run();
    }
    return Optional.ofNullable(next);
  }
}
