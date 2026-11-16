import 'dart:convert' show utf8;

import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/widget/appbar.dart';

class TransferScreen extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(_) => Scaffold(
    appBar: appBar(title: const Text('Transfer')),
    body: fileRequests.watch((_) {
      if (fileRequests.value.isEmpty) {
        return const Center(
          child: Column(
            mainAxisSize: .min,
            spacing: 12,
            children: [
              Icon(Icons.inbox_outlined, size: 64, color: Colors.grey),
              Text(
                'The history is empty',
                style: TextStyle(fontSize: 16, color: Colors.grey),
              ),
            ],
          ),
        );
      }

      return ListView.builder(
        itemCount: fileRequests.value.length,
        itemBuilder: (_, index) {
          final r = fileRequests.value[index];
          final fileName = utf8.decode(r.fileInfo.name, allowMalformed: true);

          return ListTile(
            isThreeLine: true,
            leading: const Icon(Icons.download),
            title: Text(fileName, maxLines: 1, overflow: .ellipsis),
            subtitle: Column(
              crossAxisAlignment: .start,
              children: [
                Text(formatBytes(r.fileInfo.size)),
                Text(formatDate(r.fileInfo.lastModified)),
              ],
            ),
          );
        },
      );
    }),
  );
}

String formatBytes(BigInt bytes) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  double size = bytes.toDouble();
  int unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit++;
  }
  return '${size.toStringAsFixed(unit == 0 ? 0 : 1)} ${units[unit]}';
}

String formatDate(BigInt millis) =>
    DateTime.fromMillisecondsSinceEpoch(millis.toInt())
        .toString()
        .substring(0, 16);
