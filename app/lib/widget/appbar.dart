import 'package:flutter/material.dart';
import 'package:localpost/page/settings.dart';

AppBar appBar(BuildContext context, {Widget? title}) => AppBar(
  title: title,
  actionsPadding: const .only(right: 12),
  actions: [
    PopupMenuButton(
      position: .under,
      itemBuilder: (_) => [
        popupMenuItem(
          icon: Icons.qr_code_rounded,
          child: const Text('Show QR'),
          onTap: () {},
        ),

        popupMenuItem(
          icon: Icons.link,
          child: const Text('Connect'),
          onTap: () {},
        ),

        popupMenuItem(
          onTap: () {},
          icon: Icons.qr_code_scanner,
          child: const Text('Scan QR'),
        ),

        popupMenuItem(
          icon: Icons.settings,
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
  required IconData icon,
  required Widget child,
  VoidCallback? onTap,
}) => PopupMenuItem(
  onTap: onTap,
  child: Row(spacing: 8, children: [Icon(icon), child]),
);
