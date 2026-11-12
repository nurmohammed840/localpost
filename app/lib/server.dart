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

    final _ = server.run();
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
