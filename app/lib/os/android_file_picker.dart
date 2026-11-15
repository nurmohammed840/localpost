import 'package:flutter/services.dart';

class PickedFile {
  const PickedFile({required this.fd, required this.name});

  final int fd;
  final String name;

  @override
  String toString() => 'PickedFile(fd: $fd, name: $name)';
}

class NativeFilePicker {
  NativeFilePicker._();

  static const MethodChannel _channel = MethodChannel('app/file_fd');

  static Future<PickedFile?> pickFile({bool writable = false}) async {
    final res = await _channel.invokeMapMethod<String, dynamic>('pickFile', {
      'writable': writable,
    });
    if (res == null) return null;

    return PickedFile(fd: res['fd'] as int, name: res['name'] as String);
  }
}
