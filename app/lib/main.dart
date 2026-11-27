import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/core/signal.dart';
import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/page/settings.dart';
import 'package:localpost/screen/home.dart';
import 'package:localpost/screen/transfer.dart';
import 'package:localpost/src/rust/frb_generated.dart';

Future main() async {
  await RustLib.init();
  runApp(const MyApp());

  await Settings.load();
  await runServer();
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(_) => Settings.theme.watch(
    (_) => MaterialApp(
      scaffoldMessengerKey: scaffoldMessengerKey,
      debugShowCheckedModeBanner: false,
      themeMode: Settings.theme.value,
      theme: .light(),
      darkTheme: .dark(),
      title: 'Local Post',
      home: const MainPage(),
    ),
  );
}

class MainPage extends StatelessWidget {
  const MainPage({super.key});

  static final screenIdx = CreateState(0);

  static const screens = [HomeScreen(), TransferScreen()];
  static const destinations = [
    NavigationDestination(
      icon: Icon(Icons.home_outlined),
      selectedIcon: Icon(Icons.home),
      label: 'Home',
    ),
    TransferNavigationIcon(),
  ];

  @override
  Widget build(_) => screenIdx.watch(
    (_) => Scaffold(
      body: IndexedStack(index: screenIdx.value, children: screens),
      bottomNavigationBar: NavigationBar(
        labelBehavior: .alwaysHide,
        selectedIndex: screenIdx.value,
        onDestinationSelected: screenIdx.set,
        destinations: destinations,
      ),
    ),
  );
}

class TransferNavigationIcon extends StatelessWidget {
  const new({super.key});

  static Widget badge(IconData icon) => downloads.watch(
    (_) => Badge(
      alignment: .topRight,
      offset: const Offset(8, -8),
      isLabelVisible: downloads.value.isNotEmpty,
      label: Text(downloads.value.length.toString()),
      child: Icon(icon),
    ),
  );

  @override
  Widget build(_) => NavigationDestination(
    icon: badge(Icons.swap_vert_outlined),
    selectedIcon: badge(Icons.swap_vert),
    label: 'Transfer',
  );
}
