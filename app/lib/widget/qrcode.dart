import 'package:flutter/material.dart';
import 'package:flutter_svg/svg.dart';
import 'package:localpost/server.dart';

class QrCode extends StatelessWidget {
  const new({super.key, required this.info});

  final ServerAddr info;

  @override
  Widget build(BuildContext context) => Center(
    child: Card(
      margin: const .all(16),
      child: Padding(
        padding: const .all(20),
        child: Column(
          mainAxisSize: .min,
          children: [
            SvgPicture.string(info.qr, width: 350, height: 350),
            const SizedBox(height: 12),
            SelectableText(info.addr, textAlign: .center),
          ],
        ),
      ),
    ),
  );
}
