import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/signal.dart';
import 'package:localpost/src/rust/api/bridge.dart';

CreateState<String?> serverAddr = CreateState(null);

Future runServer(int port) async {
  try {
    final (server, addr) = await Server.bind(port: port);
    serverAddr.set(addr);
    
    final _ = server.run();
  } catch (error) {
    showErrorSnackBar(error);
  }
}
