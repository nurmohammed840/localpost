import 'package:flutter/material.dart';
import 'package:localpost/core/signal.dart';
import 'package:shared_preferences/shared_preferences.dart';

class Settings extends StatelessWidget {
  const new({super.key});

  static final store = SharedPreferencesAsync();

  static final theme = CreateState(ThemeMode.dark);

  static Future load() async {
    final isDark = await store.getBool("Settings.isDark") ?? true;
    theme.set(isDark == true ? .dark : .light);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
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
        ],
      ),
    );
  }
}
