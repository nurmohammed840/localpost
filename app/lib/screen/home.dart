import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/core/signal.dart';
import 'package:localpost/widget/appbar.dart';
import 'package:localpost/widget/qrcode.dart';
import 'package:localpost/widget/utils/optional.dart';

class HomeScreen extends StatelessWidget {
  const new({super.key});

  static Listenable signal = Listenable.merge([serverAddr, connections]);

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: appBar(context, title: const Text('Local Post')),
      body: Watch(
        signal: signal,
        builder: (_) => serverAddr.watch((_) {
          if (connections.value.isEmpty) {
            return serverAddr.value.mapWidget(
              (_, value) => Center(child: QrCode(info: value)),
            );
          }

          return ListView.builder(
            padding: const .all(16),
            itemCount: connections.value.length,
            itemBuilder: (_, index) =>
                ConnectionCard(connection: connections.value[index]),
          );
        }),
      ),
    );
  }
}

class ConnectionCard extends StatelessWidget {
  const new({super.key, required this.connection});

  final Connection connection;

  @override
  Widget build(BuildContext context) {
    final info = connection.systemInfo;
    final scheme = Theme.of(context).colorScheme;

    return Card(
      elevation: 0,
      margin: const .only(bottom: 10),
      shape: RoundedRectangleBorder(borderRadius: .circular(8)),
      child: Column(
        children: [
          ListTile(
            isThreeLine: true,
            leading: Icon(getOsIcon(info.osType), size: 40),
            trailing: IconButton(
              tooltip: 'Disconnect',
              onPressed: connection.endpoint.disconnect,
              icon: const Icon(Icons.link_off),
              color: scheme.error,
            ),
            title: Text(info.hostname),
            subtitle: Column(
              crossAxisAlignment: .start,
              children: [
                Text(connection.addr),
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
                onPressed: () {},
                icon: const Icon(Icons.upload_file),
                label: const Text('Send file'),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

IconData getOsIcon(String osType) => switch (osType.toLowerCase()) {
  'android' => Icons.android,
  'ios' => Icons.phone_iphone,
  'macos' => Icons.apple,
  'linux' => Icons.computer,
  'windows' => Icons.window_sharp,
  _ => Icons.devices,
};
