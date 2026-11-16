import 'dart:convert' show utf8;

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
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

          return Card(
            elevation: 0,
            clipBehavior: .antiAlias,
            margin: const .symmetric(horizontal: 12, vertical: 6),
            shape: RoundedRectangleBorder(borderRadius: .circular(8)),
            child: ListTile(
              onTap: () {},
              leading: const Icon(Icons.download, size: 32),
              title: Text(fileName, maxLines: 1, overflow: .ellipsis),
              subtitle: Column(
                crossAxisAlignment: .start,
                children: [
                  Text(formatDate(r.fileInfo.lastModified)),
                  Text(formatBytes(r.fileInfo.size)),
                ],
              ),
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

String formatDate(BigInt secs) =>
    DateFormat('d MMM yyyy • h:mm a')
        .format(DateTime.fromMillisecondsSinceEpoch(secs.toInt() * 1000));
