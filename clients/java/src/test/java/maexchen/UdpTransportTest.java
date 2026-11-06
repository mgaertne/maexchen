package maexchen;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;

class UdpTransportTest {

  @Test
  void sendsDatagramsToTheServer() throws IOException {
    try (var server = new DatagramSocket(0, InetAddress.getLoopbackAddress());
        var transport =
            new UdpTransport(
                InetAddress.getLoopbackAddress().getHostAddress(), server.getLocalPort())) {
      server.setSoTimeout(2000);

      transport.send("REGISTER;anna");

      var packet = new DatagramPacket(new byte[64], 64);
      server.receive(packet);
      assertEquals(
          "REGISTER;anna",
          new String(packet.getData(), 0, packet.getLength(), StandardCharsets.UTF_8));
    }
  }

  @Test
  void receivesDatagramsFromTheServer() throws IOException {
    try (var server = new DatagramSocket(0, InetAddress.getLoopbackAddress());
        var transport =
            new UdpTransport(
                InetAddress.getLoopbackAddress().getHostAddress(), server.getLocalPort())) {
      server.setSoTimeout(2000);
      transport.send("hello");
      var request = new DatagramPacket(new byte[64], 64);
      server.receive(request);

      byte[] reply = "REGISTERED;1".getBytes(StandardCharsets.UTF_8);
      server.send(new DatagramPacket(reply, reply.length, request.getSocketAddress()));

      assertEquals("REGISTERED;1", receiveWithRetry(transport));
    }
  }

  @Test
  void receiveReturnsEmptyOnTimeout() throws IOException {
    try (var server = new DatagramSocket(0, InetAddress.getLoopbackAddress());
        var transport =
            new UdpTransport(
                InetAddress.getLoopbackAddress().getHostAddress(), server.getLocalPort())) {
      assertTrue(transport.receive().isEmpty());
    }
  }

  private static String receiveWithRetry(UdpTransport transport) throws IOException {
    for (int attempt = 0; attempt < 8; attempt++) {
      var message = transport.receive();
      if (message.isPresent()) {
        return message.get();
      }
    }
    throw new AssertionError("no datagram received");
  }
}
