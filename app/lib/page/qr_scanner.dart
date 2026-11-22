import 'package:flutter/material.dart';
import 'package:localpost/core/lifecycle.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

class QRScanner extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(ctx) => Lifecycle(
    initState: () => MobileScannerController(
      formats: const [.qrCode],
      detectionSpeed: .noDuplicates,
    ),
    dispose: (controller) => controller.dispose(),
    builder: (_, controller) => Scaffold(
      appBar: AppBar(
        title: const Text('Scan QR'),
        actionsPadding: const .only(right: 16),
        actions: [
          IconButton(
            icon: const Icon(Icons.cameraswitch),
            onPressed: controller.switchCamera,
          ),
          IconButton(
            icon: const Icon(Icons.flash_on),
            onPressed: controller.toggleTorch,
          ),
        ],
      ),
      body: MobileScanner(
        controller: controller,
        onDetect: (capture) {
          if (ModalRoute.of(ctx)?.isCurrent != true) return;

          final value = capture.barcodes
              .map((b) => b.rawValue)
              .whereType<String>()
              .firstOrNull
              ?.trim();

          if (value == null || value.isEmpty) return;
          Navigator.of(ctx).pop(value);
        },
      ),
    ),
  );
}
