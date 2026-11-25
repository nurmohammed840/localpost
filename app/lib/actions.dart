import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/dialog/prompt.dart';
import 'package:localpost/page/qr_scanner.dart';
import 'package:localpost/utils/input_formatter.dart';

Future connect(BuildContext ctx, String ipAddr) async {
  final addr = await promptIPAddr(ctx, ipAddr: ipAddr.split(':')[0]);
  if (addr == null) {
    return;
  }
  await openConnection(addr);
}

Future scanQR(BuildContext ctx) async {
  final result = await Navigator.of(ctx)
      .push<String>(MaterialPageRoute(builder: (_) => const QRScanner()));

  if (result == null) {
    return;
  }

  final part = result.split(':');
  if (part.length != 2) {
    return;
  }

  final addr = tryParseLocalAddr(part[0], part[1]);
  if (addr == null) {
    return showErrorSnackBar('Invalid address: $result');
  }

  await openConnection(addr);
}
