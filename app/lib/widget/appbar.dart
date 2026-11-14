import 'package:flutter/material.dart';
import 'package:localpost/actions.dart';
import 'package:localpost/api.dart';
import 'package:localpost/page/settings.dart';
import 'package:localpost/widget/qrcode.dart';

AppBar appBar(BuildContext context, {Widget? title}) => AppBar(
  title: title,
  actionsPadding: const .only(right: 12),
  actions: [
    PopupMenuButton(
      position: .under,
      itemBuilder: (context) => [
        ...menuList(context),
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

List<PopupMenuItem<dynamic>> menuList(BuildContext context) {
  if (serverAddr.value == null) {
    return [];
  }
  final ip = serverAddr.value!;
  return [
    popupMenuItem(
      icon: const Icon(Icons.qr_code_rounded),
      child: const Text('Local IP'),
      onTap: () async {
        await showPopupQRCode(context, ip);
      },
    ),

    popupMenuItem(
      icon: const Icon(Icons.link),
      child: const Text('Connect'),
      onTap: () async {
        await connect(context, ip.addr);
      },
    ),

    popupMenuItem(
      icon: const Icon(Icons.qr_code_scanner),
      child: const Text('Scan QR'),
      onTap: scanQR,
    ),
  ];
}

PopupMenuItem popupMenuItem({
  required Icon icon,
  required Widget child,
  VoidCallback? onTap,
}) => PopupMenuItem(
  onTap: onTap,
  child: Row(spacing: 8, children: [icon, child]),
);
