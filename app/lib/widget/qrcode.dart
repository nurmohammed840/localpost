import 'package:flutter/material.dart';
import 'package:flutter_svg/svg.dart';
import 'package:localpost/dialog/prompt.dart';
import 'package:localpost/server.dart';

class QrCode extends StatelessWidget {
  const new({super.key, required this.info});

  final ServerAddr info;

  Future connect(BuildContext context, String ipAddr) async {
    await promptIPAddr(context, ipAddr: ipAddr.split(':')[0]);
  }

  void scanQR() {}

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(maxWidth: 700, maxHeight: 800),
    child: Card(
      elevation: 0,
      margin: const .all(16),
      child: Padding(
        padding: const .all(20),
        child: Column(
          spacing: 12,
          mainAxisSize: .min,
          children: [
            Expanded(child: SvgPicture.string(info.qr)),

            SelectableText(info.addr, textAlign: .center),

            Column(
              crossAxisAlignment: .stretch,
              spacing: 12,
              children: [
                FilledButton.icon(
                  icon: const Icon(Icons.qr_code_scanner),
                  label: const Text('Scan QR'),
                  onPressed: scanQR,
                ),
                OutlinedButton.icon(
                  icon: const Icon(Icons.link),
                  label: const Text('Connect'),
                  onPressed: () async {
                    await connect(context, info.addr);
                  },
                ),
              ],
            ),
          ],
        ),
      ),
    ),
  );
}
