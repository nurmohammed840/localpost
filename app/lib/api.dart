import 'package:localpost/core/signal.dart';
import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/src/rust/api/bridge.dart';

class const ServerAddr({required final String addr, required final String qr});

class const Connection({
  required final String addr,
  required final ResEndpoint endpoint,
  required final SystemInfo systemInfo,
});

CreateState<ServerAddr?> serverAddr = CreateState(null);
CreateState<List<Connection>> connections = CreateState([]);

Future runServer(int port) async {
  try {
    final (server, addr, qr) = await Server.bind(port: port);
    serverAddr.set(ServerAddr(addr: addr, qr: qr));

    server.run().listen(
      cancelOnError: false,
      onError: (error, _) {
        showErrorSnackBar(error);
      },
      onDone: () {
        showErrorSnackBar('Server stopped');
      },
      (conn) async {
        final (connection, systemInfo) = conn;
        try {
          final (endpoint, addr) = await connection.handshake();

          connections.update((list) {
            list.add(
              Connection(
                addr: addr,
                endpoint: endpoint,
                systemInfo: systemInfo,
              ),
            );
          });
          await acceptRequests(endpoint);
        } catch (error) {
          showErrorSnackBar(error);
        }
      },
    );
  } catch (error) {
    showErrorSnackBar(error);
  }
}

Future openConnection(String addr, {int timeoutMs = 3 * 1000}) async {
  try {
    final (endpoint, systemInfo) = await ResEndpoint.open(
      addr: addr,
      timeoutMs: BigInt.from(timeoutMs),
    );

    connections.update((list) {
      list.add(
        Connection(addr: addr, endpoint: endpoint, systemInfo: systemInfo),
      );
    });
    await acceptRequests(endpoint);
  } catch (error) {
    showErrorSnackBar(error);
  }
}

Future acceptRequests(ResEndpoint endpoint) async {
  endpoint.acceptRequests().listen(
    cancelOnError: true,
    onError: (error, _) {
      showErrorSnackBar(error);
    },
    onDone: () {
      // Closed. Remove Endpoint.
    },
    (fileRequest) async {
      try {
        final fileInfo = await fileRequest.fileInfo();
      } catch (error) {
        // print(error);
      }
    },
  );
}
