import 'package:flutter/material.dart';
import 'package:localpost/actions.dart';
import 'package:localpost/page/settings.dart';

AppBar appBar(BuildContext context, {Widget? title}) => AppBar(
  title: title,
  actionsPadding: const .only(right: 12),
  actions: [
    PopupMenuButton(
      position: .under,
      itemBuilder: (context) => [
        popupMenuItem(
          icon: const Icon(Icons.qr_code_rounded),
          child: const Text('Show QR'),
          onTap: () {},
        ),

        popupMenuItem(
          icon: const Icon(Icons.link),
          child: const Text('Connect'),
          onTap: () async {
            await connect(context, "192.168.0.1");
          },
        ),

        popupMenuItem(
          icon: const Icon(Icons.qr_code_scanner),
          child: const Text('Scan QR'),
          onTap: () {},
        ),

        popupMenuItem(
          icon: const Icon(Icons.settings),
          child: const Text('Settings'),
          onTap: () {
            Navigator.push(
              context,
              MaterialPageRoute(builder: (_) => const Settings()),
            );
          },
        ),
      ],
    ),
  ],
);

PopupMenuItem popupMenuItem({
  required Icon icon,
  required Widget child,
  VoidCallback? onTap,
}) => PopupMenuItem(
  onTap: onTap,
  child: Row(spacing: 8, children: [icon, child]),
);
