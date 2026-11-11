import 'package:flutter/material.dart';
import 'package:localpost/server.dart';
import 'package:localpost/widget/appbar.dart';
import 'package:localpost/widget/qrcode.dart';

class HomeScreen extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: appBar(context, title: const Text('Local Post')),
      body: serverAddr.watch((_) {
        if (serverAddr.value == null) {
          return const SizedBox.shrink();
        }
        return QrCode(info: serverAddr.value!);
      }),
    );
  }
}
