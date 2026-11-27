import 'package:flutter/material.dart';
import 'package:localpost/core/signal.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:url_launcher/url_launcher.dart';

const applicationLegalese = "© 2026 Nur";

class Settings extends StatelessWidget {
  const new({super.key});

  static final store = SharedPreferencesAsync();

  static final theme = CreateState(ThemeMode.dark);
  static final quickReceive = CreateState(true);

  static Future load() async {
    final isDark = await store.getBool("Settings.isDark") ?? true;
    final isQuickReceive = await store.getBool("Settings.quickReceive") ?? true;

    theme.set(isDark == true ? .dark : .light);
    quickReceive.set(isQuickReceive);
  }

  @override
  Widget build(ctx) => Scaffold(
    appBar: AppBar(title: const Text("Settings")),
    body: ListView(
      children: [
        theme.watch(
          (_) => SwitchListTile(
            secondary: const Icon(Icons.dark_mode),
            title: const Text("Dark Mode"),
            subtitle: const Text("Use dark theme"),
            value: theme.value == .dark,
            onChanged: (value) async {
              theme.set(value ? .dark : .light);
              return store.setBool("Settings.isDark", value);
            },
          ),
        ),

        quickReceive.watch(
          (_) => SwitchListTile(
            secondary: const Icon(Icons.file_download_outlined),
            title: const Text('Quick Receive'),
            subtitle: const Text(
              'Automatically start downloading received files',
            ),
            value: quickReceive.value,
            onChanged: (value) async {
              quickReceive.set(value);
              return store.setBool("Settings.quickReceive", value);
            },
          ),
        ),

        const Divider(),

        ListTile(
          leading: const Icon(Icons.code),
          title: const Text("Source code"),
          subtitle: const Text("View repository on GitHub"),
          onTap: () => launchUrl(
            mode: .externalApplication,
            Uri.parse("https://github.com/nurmohammed840/localpost"),
          ),
        ),

        ListTile(
          leading: const Icon(Icons.question_mark),
          title: const Text("About"),
          subtitle: const Text("App info & licenses"),
          onTap: () {
            showAboutDialog(
              context: ctx,
              applicationVersion: "0.1.0",
              applicationLegalese: applicationLegalese,
            );
          },
        ),
      ],
    ),
  );
}
