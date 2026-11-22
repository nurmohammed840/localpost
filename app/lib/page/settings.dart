import 'package:flutter/material.dart';
import 'package:localpost/core/signal.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:url_launcher/url_launcher.dart';

const applicationLegalese = "© 2026 Nur";

class Settings extends StatelessWidget {
  const new({super.key});

  static final store = SharedPreferencesAsync();

  static final theme = CreateState(ThemeMode.dark);

  static Future load() async {
    final isDark = await store.getBool("Settings.isDark") ?? true;
    theme.set(isDark == true ? .dark : .light);
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text("Settings")),
    body: ListView(
      children: [
        theme.watch(
          (_) => SwitchListTile(
            title: const Text("Dark Mode"),
            subtitle: const Text("Use dark theme"),
            secondary: const Icon(Icons.dark_mode),
            value: theme.value == .dark,
            onChanged: (value) async {
              theme.set(value ? .dark : .light);
              return store.setBool("Settings.isDark", value);
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
              context: context,
              applicationVersion: "0.1.0",
              applicationLegalese: applicationLegalese,
            );
          },
        ),
      ],
    ),
  );
}
