import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/signal.dart';
import 'package:localpost/src/rust/api/bridge.dart';

class ServerAddr {
  const ServerAddr({required this.addr, required this.qr});
  final String addr;
  final String qr;
}

CreateState<ServerAddr?> serverAddr = CreateState(null);

Future runServer(int port) async {
  try {
    final (server, addr, qr) = await Server.bind(port: port);
    serverAddr.set(ServerAddr(addr: addr, qr: qr));

    server.run().listen(
      (event) async {
        final (connection, systemInfo) = event;

        print(systemInfo);

        try {
          final (endpoint, addr) = await connection.handshake();
          print(addr);
        } catch (error) {
          showErrorSnackBar(error);
        }
      },
      onError: (error, _) {
        showErrorSnackBar(error);
      },
      onDone: () {
        showErrorSnackBar('Server stopped');
      },
    );
  } catch (error) {
    showErrorSnackBar(error);
  }
}

Future openConnection(String ip, int port) async {
  try {
    final (endpoint, info) = await ResEndpoint.open(
      addr: "$ip:$port",
      timeoutMs: BigInt.from(3 * 1000),
    );
    print(info);
  } catch (error) {
    showErrorSnackBar(error);
  }
}
