import 'package:flutter/material.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

final scaffoldMessengerKey = GlobalKey<ScaffoldMessengerState>();

void showSnackBar(Widget content) {
  scaffoldMessengerKey.currentState?.showSnackBar(
    SnackBar(
      content: content,
      behavior: .floating,
      showCloseIcon: true,
      duration: const Duration(seconds: 7),
    ),
  );
}

void showErrorSnackBar(Object error) {
  final message = formatErrorMessage(error);
  showSnackBar(SelectableText(message));
}

String formatErrorMessage(Object error) {
  return error is AnyhowException ? error.message : error.toString();
}
