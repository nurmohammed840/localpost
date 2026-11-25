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
    TransferIcon(),
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

class TransferIcon extends StatelessWidget {
  const new({super.key});

  static final count = CreateState(0);

  Widget badge(IconData icon) => Badge(
    alignment: .topRight,
    offset: const Offset(8, -8),
    isLabelVisible: count.value > 0,
    label: Text(count.value.toString()),
    child: Icon(icon),
  );

  @override
  Widget build(BuildContext context) => count.watch(
    (_) => NavigationDestination(
      icon: badge(Icons.swap_vert_outlined),
      selectedIcon: badge(Icons.swap_vert),
      label: 'Transfer',
    ),
  );
}
