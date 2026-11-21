import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/core/signal.dart';
import 'package:localpost/src/rust/api/bridge.dart';
import 'package:localpost/widget/appbar.dart';
import 'package:localpost/widget/qrcode.dart';

class HomeScreen extends StatelessWidget {
  const new({super.key});

  static Listenable signal = Listenable.merge([serverState, connections]);

  @override
  Widget build(_) => Scaffold(
    appBar: appBar(title: const Text('Local Post')),
    body: Watch(
      signal: signal,
      builder: (_) => switch (serverState.value) {
        ServerLoading() => const Center(child: CircularProgressIndicator()),
        ServerClosed(:final message) => ServerClosedView(message: message),
        final ServerAddr ip =>
          connections.value.isEmpty
              ? Center(child: QrCode(ip: ip))
              : ConnectionList(list: connections.value),
      },
    ),
  );
}

class ServerClosedView extends StatelessWidget {
  const new({super.key, required this.message});

  final String message;

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisSize: .min,
      spacing: 16,
      children: [
        Icon(
          Icons.wifi_off_rounded,
          size: 64,
          color: Theme.of(context).colorScheme.error,
        ),
        Text(message, textAlign: .center),
        FilledButton.icon(
          onPressed: () {},
          icon: const Icon(Icons.refresh),
          label: const Text('Reload'),
        ),
      ],
    ),
  );
}

class ConnectionList extends StatelessWidget {
  const new({super.key, required this.list});

  final List<Connection> list;

  @override
  Widget build(BuildContext context) => ListView.builder(
    padding: const .all(16),
    itemCount: list.length,
    itemBuilder: (context, index) => Card(
      elevation: 0,
      margin: const .only(bottom: 10),
      shape: RoundedRectangleBorder(borderRadius: .circular(8)),
      child: ConnectionWidget(
        connection: list[index],
        info: list[index].systemInfo,
        scheme: Theme.of(context).colorScheme,
      ),
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
