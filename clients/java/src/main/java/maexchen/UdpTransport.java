package maexchen;

import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;
import java.net.SocketTimeoutException;
import java.nio.charset.StandardCharsets;
import java.util.Optional;

/** Talks to the server with UDP datagrams. */
public final class UdpTransport implements Transport, AutoCloseable {

  private static final int POLL_INTERVAL_MILLIS = 250;
  private static final int MAX_DATAGRAM_SIZE = 2048;

  private final DatagramSocket socket;

  /** Opens a socket towards the server at {@code host}:{@code port}. */
  public UdpTransport(String host, int port) throws IOException {
    socket = new DatagramSocket();
    socket.connect(InetAddress.getByName(host), port);
    socket.setSoTimeout(POLL_INTERVAL_MILLIS);
  }

  @Override
  public void send(String message) throws IOException {
    byte[] bytes = message.getBytes(StandardCharsets.UTF_8);
    socket.send(new DatagramPacket(bytes, bytes.length));
  }

  @Override
  public Optional<String> receive() throws IOException {
    var packet = new DatagramPacket(new byte[MAX_DATAGRAM_SIZE], MAX_DATAGRAM_SIZE);
    try {
      socket.receive(packet);
    } catch (SocketTimeoutException timeout) {
      return Optional.empty();
    }
    return Optional.of(new String(packet.getData(), 0, packet.getLength(), StandardCharsets.UTF_8));
  }

  @Override
  public void close() {
    socket.close();
  }
}
