import 'package:flutter/material.dart';
import 'package:flutter_svg/svg.dart';
import 'package:localpost/actions.dart';
import 'package:localpost/api.dart';

class QrCode extends StatelessWidget {
  const new({super.key, required this.info});

  final ServerAddr info;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(maxWidth: 700, maxHeight: 800),
    child: Card(
      elevation: 0,
      margin: const .all(16),
      child: Padding(
        padding: const .all(20),
        child: QRCodeWidget(info: info),
      ),
    ),
  );
}

class QRCodeWidget extends StatelessWidget {
  const new({super.key, required this.info});

  final ServerAddr info;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: .min,
    spacing: 12,
    children: [
      Expanded(child: SvgPicture.string(info.qr)),

      SelectableText(info.addr, textAlign: .center),

      Column(
        spacing: 12,
        crossAxisAlignment: .stretch,
        children: [
          FilledButton.icon(
            icon: const Icon(Icons.link),
            label: const Text('Connect'),
            onPressed: () async {
              await connect(context, info.addr);
            },
          ),
          OutlinedButton.icon(
            icon: const Icon(Icons.qr_code_scanner),
            label: const Text('Scan QR'),
            onPressed: scanQR,
          ),
        ],
      ),
    ],
  );
}

void showPopupQRCode() {}
