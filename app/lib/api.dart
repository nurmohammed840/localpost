import 'dart:convert' show utf8;
import 'dart:io' show Platform;

import 'package:flutter/foundation.dart';
import 'package:localpost/core/signal.dart';
import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/os/android_file_picker.dart';
import 'package:localpost/src/rust/api/bridge.dart';
import 'package:localpost/utils/format.dart';

CreateState<ServerState> serverState = CreateState(ServerLoading());
CreateState<List<Connection>> connections = CreateState([]);
CreateState<List<FileUpload>> uploads = CreateState([]);
CreateState<List<FileDownload>> downloads = CreateState([]);

sealed class ServerState {}

class ServerLoading extends ServerState {}

class ServerClosed(final String message) extends ServerState {}

class ServerAddr(final String addr, final String qr) extends ServerState {
  int get port => int.parse(addr.split(':')[1]);
}

class const Connection(
  final String peerServerAddr,
  final String peerAddr,
  final ResEndpoint endpoint,
  final SystemInfo systemInfo,
) {
  Future disconnect() async {
    try {
      connections.update((list) {
        list.removeWhere((connection) => connection.peerAddr == peerAddr);
      });

      await endpoint.disconnect();
    } catch (error) {
      showErrorSnackBar(error);
    } finally {
      endpoint.dispose();
    }
  }

  Future sendFile() async {
    final status = ResSendStatus();

    try {
      final file = await _pickResFile();
      if (file == null) return;

      final info = file.fileInfo();
      uploads.update(
        (list) => list.add(FileUpload(FormattedFileInfo(info), status)),
      );

      await endpoint.sendFile(file: file, status: status);
    } catch (error) {
      showErrorSnackBar(error);
    }
  }
}

class FormattedFileInfo(final FileInfo fileInfo) {
  final fileName = utf8.decode(fileInfo.name, allowMalformed: true);
  final lastModified = formatDate(fileInfo.lastModified);
  final size = formatBytes(fileInfo.size);
}

class FileUpload(
  final FormattedFileInfo fileInfo,
  final ResSendStatus status,
) {}

class FileDownload(
  final FormattedFileInfo fileInfo,
  final ResFileRequest request,
) {
  final status = CreateState<ResRecvStatus?>(null);

  bool removeFromList() => downloads.update((list) => list.remove(this));

  Future reject() async {
    try {
      removeFromList();
      await request.closeWithoutResponse();
    } catch (error) {
      showErrorSnackBar(error);
    }
  }

  Future accept() async {
    final recvStatus = ResRecvStatus();
    status.set(recvStatus);
    // request.receive(path: "path", status: recvStatus);
  }
}

Future runServer({int port = 53971}) async {
  try {
    if (serverState.value is ServerAddr) {
      throw 'Server is already running';
    }

    serverState.set(ServerLoading());
    final (server, addr, qr) = await Server.bind(port: port);
    serverState.set(ServerAddr(addr, qr));

    server.run().listen(
      cancelOnError: false,
      onError: (error, _) {
        showErrorSnackBar(error);
      },
      onDone: () {
        serverState.set(ServerClosed("Server stopped"));
      },
      (conn) async {
        final (connection, systemInfo) = conn;
        try {
          final (endpoint, peerAddr) = await connection.handshake();
          final peerServerAddr = "${peerAddr.split(':')[0]}:${systemInfo.port}";

          final conn = Connection(
            peerServerAddr,
            peerAddr,
            endpoint,
            systemInfo,
          );

          connections.update((list) => list.add(conn));
          acceptRequests(conn);
        } catch (error) {
          showErrorSnackBar(error);
        }
      },
    );
  } catch (error) {
    serverState.set(ServerClosed(formatErrorMessage(error)));
  }
}

Future openConnection(String peerServerAddr, {int timeoutMs = 3 * 1000}) async {
  try {
    if (connections.value.any(
      (connection) => connection.peerServerAddr == peerServerAddr,
    )) {
      throw 'Already connected to $peerServerAddr';
    }

    final myServer = switch (serverState.value) {
      final ServerAddr ip => ip,
      _ => throw 'Server is not running',
    };

    if (kReleaseMode && peerServerAddr == myServer.addr) {
      throw "Cannot connect to self";
    }

    final (endpoint, systemInfo) = await ResEndpoint.open(
      addr: peerServerAddr,
      timeoutMs: BigInt.from(timeoutMs),
      serverPort: myServer.port,
    );

    final conn = Connection(
      peerServerAddr,
      peerServerAddr,
      endpoint,
      systemInfo,
    );

    connections.update((list) => list.add(conn));
    acceptRequests(conn);
  } catch (error) {
    showErrorSnackBar(error);
  }
}

void acceptRequests(Connection endpoint) {
  endpoint.endpoint.acceptRequests().listen(
    onDone: () => endpoint.disconnect(),
    cancelOnError: true,
    onError: (error, _) {
      showErrorSnackBar(error);
      return endpoint.disconnect();
    },
    (request) async {
      try {
        final info = await request.fileInfo();
        downloads.update((list) {
          list.add(FileDownload(FormattedFileInfo(info), request));
        });
      } catch (error) {
        showErrorSnackBar(error);
      }
    },
  );
}

// -----------------------------------------------

Future<ResFile?> _pickResFile() async {
  const frameSize = 1500 - (20 + 8);

  if (Platform.isAndroid) {
    final picked = await AndroidFilePicker.pickFile();
    if (picked == null) return null;

    return ResFile.fromFd(
      fd: picked.fd,
      name: picked.name,
      frameSize: frameSize,
    );
  }

  final path = await pickFile();
  if (path == null) return null;
  return ResFile.fromPath(path: path, frameSize: frameSize);
}
