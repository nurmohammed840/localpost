import 'dart:io';

import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/dialog/prompt.dart';
import 'package:localpost/src/rust/api/bridge.dart';

Future connect(BuildContext context, String ipAddr) async {
  final addr = await promptIPAddr(context, ipAddr: ipAddr.split(':')[0]);
  if (addr == null) {
    return;
  }
  await openConnection(addr);
}

void scanQR() {}

Future filePicker() async {
  if (Platform.isAndroid) {}
  final _ = await pickFile();
}
