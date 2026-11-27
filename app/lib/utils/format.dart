import 'package:intl/intl.dart';

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
