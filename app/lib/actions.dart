import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/dialog/prompt.dart';
import 'package:localpost/page/qr_scanner.dart';

Future connect(BuildContext context, String ipAddr) async {
  final addr = await promptIPAddr(context, ipAddr: ipAddr.split(':')[0]);
  if (addr == null) {
    return;
  }
  await openConnection(addr);
}

Future scanQR(BuildContext context) async {
  final result = await Navigator.of(context)
      .push<String>(MaterialPageRoute(builder: (_) => const QRScanner()));
}
