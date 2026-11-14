import 'package:localpost/core/signal.dart';
import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/src/rust/api/bridge.dart';

class const ServerAddr(final String addr, final String qr) {
  int port() {
    return int.parse(addr.split(':')[1]);
  }
}

class const Connection(
  final String serverAddr,
  final ResEndpoint endpoint,
  final SystemInfo systemInfo,
) {
  Future disconnect() async {
    try {
      connections.update((list) {
        list.removeWhere((connection) => connection.serverAddr == serverAddr);
      });
      await endpoint.disconnect();
    } catch (error) {
      showErrorSnackBar(error);
    } finally {
      endpoint.dispose();
    }
  }

  Future sendFile() async {
    // endpoint.sendFile(path: "path", frameSize: 1500 - (20 + 8));
  }
}

class FileRequest(final FileInfo fileInfo, final ResFile request) {
  final status = CreateState<ResRecvStatus?>(null);

  Future receive() async {
    final recvStatus = ResRecvStatus();
    status.set(recvStatus);
    // request.receive(path: "path", status: recvStatus);
  }
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

          final conn = Connection(addr, endpoint, systemInfo);
          connections.update((list) => list.add(conn));
          acceptRequests(conn);
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
    final port = serverAddr.value?.port();
    if (port == null) {
      throw "No server is running";
    }

    final (endpoint, systemInfo) = await ResEndpoint.open(
      addr: addr,
      timeoutMs: BigInt.from(timeoutMs),
      serverPort: port,
    );

    final conn = Connection(addr, endpoint, systemInfo);
    connections.update((list) => list.add(conn));
    acceptRequests(conn);
  } catch (error) {
    showErrorSnackBar(error);
  }
}

void removeConnection(String addr) {
  connections.update((list) {
    list.removeWhere((connection) => connection.serverAddr == addr);
  });
}

void acceptRequests(Connection endpoint) {
  endpoint.endpoint.acceptRequests().listen(
    onDone: () => endpoint.disconnect(),
    cancelOnError: true,
    onError: (error, _) {
      showErrorSnackBar(error);
      return endpoint.disconnect();
    },
    (request) async {
      try {
        final fileInfo = await request.fileInfo();
        fileRequests.update((list) {
          list.add(FileRequest(fileInfo, request));
        });
      } catch (error) {
        showErrorSnackBar(error);
      }
    },
  );
}
