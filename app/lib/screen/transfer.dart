import 'dart:convert' show utf8;

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:localpost/api.dart';
import 'package:localpost/utils/file_icon.dart';
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
        itemBuilder: (ctx, index) {
          final req = fileRequests.value[index];
          final fileName = utf8.decode(req.fileInfo.name, allowMalformed: true);
          return FileCard(fileName: fileName, req: req);
        },
      );
    }),
  );
}

class FileCard extends StatelessWidget {
  const new({super.key, required this.fileName, required this.req});

  final String fileName;
  final FileRequest req;

  @override
  Widget build(_) => Card(
    elevation: 0,
    clipBehavior: .antiAlias,
    margin: const .symmetric(horizontal: 12, vertical: 6),
    shape: RoundedRectangleBorder(borderRadius: .circular(8)),
    child: ListTile(
      onTap: () {},
      leading: Icon(fileIcon(fileName), size: 32),
      title: Text(fileName, maxLines: 1, overflow: .ellipsis),
      subtitle: Column(
        crossAxisAlignment: .start,
        children: [
          Text(formatDate(req.fileInfo.lastModified)),
          Text(formatBytes(req.fileInfo.size)),
        ],
      ),
      trailing: const AcceptRejectButtons(),
    ),
  );
}

class AcceptRejectButtons extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(ctx) {
    final scheme = Theme.of(ctx).colorScheme;

    return Row(
      mainAxisSize: .min,
      children: [
        IconButton(
          tooltip: 'Reject',
          color: scheme.error,
          icon: const Icon(Icons.close),
          onPressed: () {},
        ),
        IconButton(
          tooltip: 'Accept',
          color: scheme.primary,
          icon: const Icon(Icons.check),
          onPressed: () {},
        ),
      ],
    );
  }
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
