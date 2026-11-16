import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/core/signal.dart';
import 'package:localpost/src/rust/api/bridge.dart';
import 'package:localpost/widget/appbar.dart';
import 'package:localpost/widget/qrcode.dart';
import 'package:localpost/widget/utils/optional.dart';

class HomeScreen extends StatelessWidget {
  const new({super.key});

  static Listenable signal = Listenable.merge([serverAddr, connections]);

  @override
  Widget build(_) => Scaffold(
    appBar: appBar(title: const Text('Local Post')),
    body: Watch(
      signal: signal,
      builder: (_) => serverAddr.watch((_) {
        if (connections.value.isEmpty) {
          return serverAddr.value.mapWidget(
            (_, ip) => Center(child: QrCode(ip: ip)),
          );
        }

        return ListView.builder(
          padding: const .all(16),
          itemCount: connections.value.length,
          itemBuilder: (context, index) => Card(
            elevation: 0,
            margin: const .only(bottom: 10),
            shape: RoundedRectangleBorder(borderRadius: .circular(8)),
            child: ConnectionWidget(
              connection: connections.value[index],
              info: connections.value[index].systemInfo,
              scheme: Theme.of(context).colorScheme,
            ),
          ),
        );
      }),
    ),
  );
}

class ConnectionWidget extends StatelessWidget {
  const new({
    super.key,
    required this.info,
    required this.connection,
    required this.scheme,
  });

  final SystemInfo info;
  final Connection connection;
  final ColorScheme scheme;

  @override
  Widget build(BuildContext context) => Column(
    children: [
      ListTile(
        isThreeLine: true,
        leading: Icon(getOsIcon(info.osType), size: 40),
        trailing: IconButton(
          tooltip: 'Disconnect',
          onPressed: connection.disconnect,
          icon: const Icon(Icons.link_off),
          color: scheme.error,
        ),

        title: Text(info.hostname),
        subtitle: Column(
          crossAxisAlignment: .start,
          children: [
            Text(connection.serverAddr),
            Text('${info.osName} ${info.osVersion}'),
            Text('${info.osType} • ${info.architecture}'),
          ],
        ),
      ),

      Padding(
        padding: const .all(8.0),
        child: Align(
          alignment: .centerRight,
          child: FilledButton.tonalIcon(
            icon: const Icon(Icons.upload_file),
            label: const Text('Send file'),
            onPressed: connection.sendFile,
          ),
        ),
      ),
    ],
  );
}

IconData getOsIcon(String osType) => switch (osType.toLowerCase()) {
  'android' => Icons.android,
  'ios' => Icons.phone_iphone,
  'macos' => Icons.apple,
  'linux' => Icons.computer,
  'windows' => Icons.window_sharp,
  _ => Icons.devices,
};
