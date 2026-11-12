import 'dart:io';

import 'package:flutter/services.dart';

class IPv4Formatter extends TextInputFormatter {
  @override
  TextEditingValue formatEditUpdate(
    TextEditingValue old,
    TextEditingValue val,
  ) {
    final text = val.text;
    if (text.isEmpty) return val;
    if (!RegExp(r'^[0-9.]+$').hasMatch(text)) return old;
    if (text.startsWith('.') || text.contains('..')) return old;

    final parts = text.split('.');
    if (parts.length > 4) return old;

    for (final p in parts) {
      if (p.length > 3) return old;
      if (p.length > 1 && p.startsWith('0')) return old;
      if (p.isNotEmpty && int.parse(p) > 255) return old;
    }
    return val;
  }
}

class PortFormatter extends TextInputFormatter {
  @override
  TextEditingValue formatEditUpdate(
    TextEditingValue old,
    TextEditingValue val,
  ) {
    final t = val.text;
    if (t.isEmpty) return val;
    if (!RegExp(r'^[0-9]{1,5}$').hasMatch(t)) return old;
    if (t.startsWith('0')) return old; // blocks "0" and "080"
    if (int.parse(t) > 65535) return old;
    return val;
  }
}

(String, int)? parseLocalAddr(String ip, String port) {
  final isV4 = InternetAddress.tryParse(ip)?.type == InternetAddressType.IPv4;
  final p = int.tryParse(port);
  if (!isV4 || p == null || p < 1 || p > 65535) return null;
  return (ip, p);
}
