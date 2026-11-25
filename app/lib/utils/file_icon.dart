// GENERATED: By LLM

import 'package:flutter/material.dart';

// dart format off
const _groups = <(IconData, List<String>)>[
  // Documents
  (Icons.picture_as_pdf, ['pdf']),
  (Icons.description, ['doc', 'docx', 'dot', 'dotx', 'odt', 'rtf', 'pages']),
  (Icons.text_snippet, ['txt', 'text', 'log']),
  (Icons.article, ['md', 'markdown', 'rst', 'tex', 'adoc']),
  (Icons.menu_book, ['epub', 'mobi', 'azw', 'azw3', 'fb2']),
  (Icons.email, ['eml', 'msg', 'mbox']),
  (Icons.contacts, ['vcf']),
  (Icons.event, ['ics', 'ical']),

  // Spreadsheets & presentations
  (Icons.table_chart, ['xls', 'xlsx', 'xlsm', 'ods', 'numbers']),
  (Icons.table_view, ['csv', 'tsv']),
  (Icons.slideshow, ['ppt', 'pptx', 'pps', 'ppsx', 'odp', 'key']), // key = Keynote

  // Images & design
  (Icons.image, ['png', 'jpg', 'jpeg', 'bmp', 'webp', 'heic', 'heif', 'tif', 'tiff', 'ico', 'avif']),
  (Icons.gif, ['gif']),
  (Icons.photo_camera, ['raw', 'cr2', 'nef', 'arw', 'dng']),
  (Icons.draw, ['svg', 'ai', 'eps']),
  (Icons.brush, ['psd', 'xcf', 'sketch', 'fig', 'xd']),
  (Icons.view_in_ar, ['obj', 'stl', 'fbx', 'gltf', 'glb', 'blend', '3ds']),
  (Icons.architecture, ['dwg', 'dxf']),
  (Icons.map, ['kml', 'kmz', 'gpx', 'shp']),

  // Video & audio
  (Icons.video_file, ['mp4', 'm4v', 'mkv', 'mov', 'avi', 'wmv', 'flv', 'webm', 'mpg', 'mpeg', '3gp']),
  (Icons.audio_file, ['mp3', 'wav', 'flac', 'aac', 'ogg', 'oga', 'm4a', 'wma', 'opus', 'aiff']),
  (Icons.piano, ['mid', 'midi']),
  (Icons.subtitles, ['srt', 'vtt', 'ass', 'sub']),

  // Archives, disk images, packages
  (Icons.folder_zip, ['zip', 'rar', '7z', 'tar', 'gz', 'tgz', 'bz2', 'xz', 'zst', 'lz']),
  (Icons.album, ['iso', 'img', 'vhd', 'vmdk']),
  (Icons.inventory_2, ['deb', 'rpm', 'appimage', 'snap', 'flatpak', 'jar', 'war']),

  // Executables & binaries
  (Icons.android, ['apk', 'aab']),
  (Icons.phone_iphone, ['ipa']),
  (Icons.desktop_windows, ['exe', 'msi']),
  (Icons.laptop_mac, ['app', 'dmg', 'pkg']),
  (Icons.terminal, ['sh', 'bash', 'zsh', 'fish', 'bat', 'cmd', 'ps1']),
  (Icons.memory, ['dll', 'so', 'dylib', 'lib', 'bin', 'o', 'a', 'wasm']),

  // Web
  (Icons.html, ['html', 'htm', 'xhtml']),
  (Icons.css, ['css', 'scss', 'sass', 'less']),
  (Icons.javascript, ['js', 'mjs', 'cjs', 'jsx', 'ts', 'tsx']),
  (Icons.php, ['php']),

  // Data & config
  (Icons.data_object, ['json', 'jsonc', 'json5', 'geojson', 'xml', 'xsd', 'xsl', 'plist']),
  (Icons.settings, ['yaml', 'yml', 'toml', 'ini', 'conf', 'cfg', 'env', 'properties']),
  (Icons.storage, ['sql', 'db', 'sqlite', 'sqlite3', 'mdb', 'accdb', 'parquet']),
  (Icons.science, ['ipynb']),

  // Source code
  (Icons.code, [
    'dart', 'rs', 'py', 'java', 'kt', 'kts', 'swift', 'c', 'h', 'cpp', 'cc', 'hpp',
    'cs', 'go', 'rb', 'lua', 'r', 'scala', 'pl', 'hs', 'ex', 'exs', 'zig', 'vue',
    'svelte', 'gradle', 'diff', 'patch',
  ]),

  // Security
  (Icons.key, ['pem', 'pub', 'ppk', 'asc']),
  (Icons.verified_user, ['crt', 'cer', 'der', 'p12', 'pfx']),
  (Icons.lock, ['enc', 'aes', 'gpg', 'lock']),

  // Misc
  (Icons.font_download, ['ttf', 'otf', 'woff', 'woff2', 'eot']),
  (Icons.download, ['torrent']),
  (Icons.link, ['url', 'webloc', 'lnk']),
  (Icons.backup, ['bak', 'bkp', 'old']),
];

// Files recognised by full name (no extension, or special dotfiles).
const _nameIcons = <String, IconData>{
  'dockerfile': Icons.build,
  'makefile': Icons.build,
  'license': Icons.gavel,
  'readme': Icons.info_outline,
  'readme.md': Icons.info_outline,
  '.gitignore': Icons.call_split,
  '.gitattributes': Icons.call_split,
  '.env': Icons.settings,
  'pubspec.yaml': Icons.flutter_dash,
};

final Map<String, IconData> _extIcons = {
  for (final (icon, exts) in _groups)
    for (final ext in exts) ext: icon,
};

IconData fileIcon(String fileName) {
  final name = fileName.split(RegExp(r'[\\/]')).last.toLowerCase();

  final byName = _nameIcons[name];
  if (byName != null) return byName;

  final dot = name.lastIndexOf('.');
  if (dot <= 0 || dot == name.length - 1) return Icons.insert_drive_file;

  return _extIcons[name.substring(dot + 1)] ?? Icons.insert_drive_file;
}