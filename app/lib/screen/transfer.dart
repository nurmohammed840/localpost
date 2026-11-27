import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/core/signal.dart';
import 'package:localpost/utils/file_icon.dart';
import 'package:localpost/widget/appbar.dart';

class TransferScreen extends StatelessWidget {
  const new({super.key});

  static Listenable signal = .merge([uploads, downloads]);

  @override
  Widget build(_) => Scaffold(
    appBar: appBar(title: const Text('Transfer')),
    body: signal.watch((_) {
      if (uploads.value.isEmpty && downloads.value.isEmpty) {
        return const Center(child: EmptyHistory());
      }

      return CustomScrollView(
        slivers: [
          if (uploads.value.isNotEmpty) ...[
            const GroupHeader(title: 'Upload'),
            SliverList.builder(
              itemCount: uploads.value.length,
              itemBuilder: (ctx, index) {
                final item = uploads.value[index];
                return FileCard(info: item.fileInfo);
                // ...
              },
            ),
          ],

          if (downloads.value.isNotEmpty) ...[
            const GroupHeader(title: 'Download'),
            SliverList.builder(
              itemCount: downloads.value.length,
              itemBuilder: (ctx, index) {
                final item = downloads.value[index];
                final status = item.status;

                return status.watch(
                  (_) => FileCard(
                    info: item.fileInfo,

                    trailing: item.status.value == null
                        ? AcceptRejectButtons(req: item)
                        : null,

                    onTap: status.value == null ? null : () {},
                  ),
                );
              },
            ),
          ],

          // ...
        ],
      );
    }),
  );
}

class EmptyHistory extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) => const Column(
    mainAxisSize: .min,
    spacing: 12,
    children: [
      Icon(Icons.inbox_outlined, size: 64, color: Colors.grey),
      Text(
        'The history is empty',
        style: TextStyle(fontSize: 16, color: Colors.grey),
      ),
    ],
  );
}

class FileCard extends StatelessWidget {
  const new({super.key, required this.info, this.trailing, this.onTap});

  final FormattedFileInfo info;
  final Widget? trailing;
  final VoidCallback? onTap;

  @override
  Widget build(_) => Card(
    elevation: 0,
    clipBehavior: .antiAlias,
    margin: const .symmetric(horizontal: 12, vertical: 6),
    shape: RoundedRectangleBorder(borderRadius: .circular(8)),
    child: ListTile(
      leading: Icon(fileIcon(info.fileName), size: 32),
      title: Text(info.fileName, maxLines: 1, overflow: .ellipsis),
      trailing: trailing,
      onTap: onTap,
      subtitle: Column(
        crossAxisAlignment: .start,
        children: [Text(info.lastModified), Text(info.size)],
      ),
    ),
  );
}

class AcceptRejectButtons extends StatelessWidget {
  const new({super.key, required this.req});

  final FileDownload req;

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
          onPressed: req.reject,
        ),
        IconButton(
          tooltip: 'Accept',
          color: scheme.primary,
          icon: const Icon(Icons.check),
          onPressed: req.accept,
        ),
      ],
    );
  }
}

class GroupHeader extends StatelessWidget {
  const new({super.key, required this.title});

  final String title;

  @override
  Widget build(ctx) => SliverToBoxAdapter(
    child: Padding(
      padding: const .fromLTRB(16, 16, 16, 4),
      child: Text(
        title,
        style: TextStyle(
          color: Theme.of(ctx).colorScheme.primary,
          fontSize: 14,
        ),
      ),
    ),
  );
}
