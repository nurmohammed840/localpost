import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:localpost/core/lifecycle.dart';
import 'package:localpost/utils/input_formatter.dart';

Future<String?> promptIPAddr(
  BuildContext ctx, {
  String ipAddr = '192.168.0.1',
  int port = 53971,
}) {
  if (kReleaseMode) {
    ipAddr = ipAddr.substring(0, ipAddr.length - 1);
  }
  final ipController = TextEditingController(text: ipAddr);
  final portController = TextEditingController(text: '$port');

  return showDialog(
    context: ctx,
    builder: (_) => Lifecycle(
      dispose: (_) {
        ipController.dispose();
        portController.dispose();
      },
      builder: (_, _) => AlertDialog(
        constraints: const BoxConstraints(minWidth: 550),
        title: const Text('Connect Device'),
        content: Column(
          mainAxisSize: .min,
          spacing: 12,
          children: [
            TextField(
              controller: ipController,
              autofocus: true,
              keyboardType: .number,
              inputFormatters: [IPv4Formatter()],
              decoration: const InputDecoration(labelText: 'IP address'),
            ),
            TextField(
              controller: portController,
              keyboardType: .number,
              inputFormatters: [PortFormatter()],
              decoration: const InputDecoration(labelText: 'Port'),
            ),
          ],
        ),
        actions: [
          ListenableBuilder(
            listenable: .merge([ipController, portController]),
            builder: (ctx, _) {
              final addr = tryParseLocalAddr(
                ipController.text,
                portController.text,
              );

              return FilledButton(
                onPressed: addr == null ? null : () => Navigator.pop(ctx, addr),
                child: const Text('Connect'),
              );
            },
          ),
        ],
      ),
    ),
  );
}
