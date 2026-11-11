import 'package:flutter/material.dart';

class TransferScreen extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Transfer'),
        actionsPadding: const .only(right: 10),
      ),
    );
  }
}
