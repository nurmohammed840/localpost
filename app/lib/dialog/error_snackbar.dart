import 'package:flutter/material.dart';

final scaffoldMessengerKey = GlobalKey<ScaffoldMessengerState>();

void showSnackBar(Widget content) {
  WidgetsBinding.instance.addPostFrameCallback((_) {
    scaffoldMessengerKey.currentState?.showSnackBar(
      SnackBar(
        content: content,
        behavior: .floating,
        showCloseIcon: true,
        duration: const Duration(seconds: 5),
      ),
    );
  });
}

void showErrorSnackBar(Object error) {
  showSnackBar(Text(error.toString()));
}
