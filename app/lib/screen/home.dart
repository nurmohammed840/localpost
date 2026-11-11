import 'package:flutter/material.dart';
import 'package:localpost/widget/appbar.dart';

class HomeScreen extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(appBar: appBar(context, title: const Text('Local Post')));
  }
}
