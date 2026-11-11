import 'package:flutter/material.dart';
import 'package:localpost/page/settings.dart';

AppBar appBar(BuildContext context, {Widget? title}) => AppBar(
  title: title,
  actionsPadding: const .only(right: 10),
  actions: [
    IconButton(
      tooltip: 'Settings',
      icon: const Icon(Icons.settings_outlined),
      onPressed: () {
        Navigator.push(
          context,
          MaterialPageRoute(builder: (_) => const Settings()),
        );
      },
    ),
  ],
);
