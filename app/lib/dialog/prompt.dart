import 'package:flutter/material.dart';
import 'package:localpost/utils.dart';

Future<String?> promptIPAddr(BuildContext context) async {
  final controller = TextEditingController(text: '192.168.0.1');

  return showDialog<String>(
    context: context,
    builder: (_) => AlertDialog(
      title: const Text('Connect Device'),
      content: TextField(
        controller: controller,
        autofocus: true,
        keyboardType: .number,
        inputFormatters: [IPv4Formatter()],
        decoration: const InputDecoration(labelText: 'IP address'),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, controller.text),
          child: const Text('Connect'),
        ),
      ],
    ),
  );
}
