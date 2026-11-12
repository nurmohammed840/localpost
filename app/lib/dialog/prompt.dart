import 'package:flutter/material.dart';
import 'package:localpost/constraints.dart';
import 'package:localpost/utils.dart';

Future<(String, int)?> promptIPAddr(
  BuildContext context, {
  String ipAddr = '192.168.0.1',
  int port = defaultPort,
}) async {
  ipAddr = ipAddr.substring(0, ipAddr.length - 1);
  final ipController = TextEditingController(text: ipAddr);
  final portController = TextEditingController(text: '$port');

  final result = await showDialog<(String, int)>(
    context: context,
    builder: (_) => AlertDialog(
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
          listenable: Listenable.merge([ipController, portController]),
          builder: (context, _) {
            final addr = parseLocalAddr(ipController.text, portController.text);

            return FilledButton(
              onPressed: addr == null
                  ? null
                  : () => Navigator.pop(context, addr),

              child: const Text('Connect'),
            );
          },
        ),
      ],
    ),
  );

  ipController.dispose();
  portController.dispose();

  return result;
}
