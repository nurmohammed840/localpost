import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:localpost/actions.dart';
import 'package:localpost/api.dart';
import 'package:localpost/core/signal.dart';
import 'package:localpost/main.dart';
import 'package:localpost/page/settings.dart';
import 'package:localpost/widget/qrcode.dart';

AppBar appBar({Widget? title}) => AppBar(
  title: title,
  actionsPadding: const .only(right: 12),
  actions: const [ActionBtn()],
);

class ActionBtn extends StatelessWidget {
  const new({super.key});

  static Listenable signal = .merge([MainPage.screenIdx, connections]);

  @override
  Widget build(ctx) => signal.watch((ctx) {
    if (MainPage.screenIdx.value == 0 && connections.value.isEmpty) {
      return IconButton(
        tooltip: 'Settings',
        icon: const Icon(Icons.settings),
        onPressed: () {
          Navigator.push(
            ctx,
            MaterialPageRoute(builder: (_) => const Settings()),
          );
        },
      );
    }

    return PopupMenuButton(
      position: .under,
      itemBuilder: (ctx) => [
        ...menuList(ctx),
        popupMenuItem(
          icon: const Icon(Icons.settings),
          child: const Text('Settings'),
          onTap: () {
            Navigator.push(
              ctx,
              MaterialPageRoute(builder: (_) => const Settings()),
            );
          },
        ),
      ],
    );
  });
}

List<PopupMenuItem<dynamic>> menuList(BuildContext ctx) {
  final ip = serverState.value;
  if (ip is! ServerAddr) {
    return [];
  }

  return [
    popupMenuItem(
      icon: const Icon(Icons.qr_code_rounded),
      child: const Text('Local IP'),
      onTap: () async {
        await showPopupQRCode(ctx, ip);
      },
    ),

    popupMenuItem(
      icon: const Icon(Icons.link),
      child: const Text('Connect'),
      onTap: () async {
        await connect(ctx, ip.addr);
      },
    ),

    if (Platform.isAndroid)
      popupMenuItem(
        icon: const Icon(Icons.qr_code_scanner),
        child: const Text('Scan QR'),
        onTap: () => scanQR(ctx),
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
