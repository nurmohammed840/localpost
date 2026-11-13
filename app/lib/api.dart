import 'package:localpost/core/signal.dart';
import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/src/rust/api/bridge.dart';

class const ServerAddr(final String addr, final String qr);

class const Connection(
  final String addr,
  final ResEndpoint endpoint,
  final SystemInfo systemInfo,
);

class FileRequest(final FileInfo fileInfo) {
  final status = CreateState<ResRecvStatus?>(null);
}

CreateState<ServerAddr?> serverAddr = CreateState(null);
CreateState<List<Connection>> connections = CreateState([]);
CreateState<List<FileRequest>> fileRequests = CreateState([]);

Future runServer(int port) async {
  try {
    final (server, addr, qr) = await Server.bind(port: port);
    serverAddr.set(ServerAddr(addr, qr));

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
            list.add(Connection(addr, endpoint, systemInfo));
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
      list.add(Connection(addr, endpoint, systemInfo));
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
        fileRequests.update((list) {
          list.add(FileRequest(fileInfo));
        });
      } catch (error) {
        showErrorSnackBar(error);
      }
    },
  );
}
