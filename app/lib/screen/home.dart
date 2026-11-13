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
            padding: const .all(12),
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
  Widget build(_) {
    final info = connection.systemInfo;

    return Card(
      margin: const .only(bottom: 10),
      child: ListTile(
        leading: const Icon(Icons.devices, size: 32),
        title: Text(info.hostname),
        subtitle: Column(
          crossAxisAlignment: .start,
          children: [
            Text(connection.addr),
            Text('${info.osName} ${info.osVersion}'),
            Text('${info.osType} • ${info.architecture}'),
          ],
        ),
        trailing: const Icon(Icons.chevron_right),
        isThreeLine: true,
      ),
    );
  }
}
