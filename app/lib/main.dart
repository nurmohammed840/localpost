import 'package:flutter/material.dart';
import 'package:localpost/api.dart';
import 'package:localpost/constraints.dart';
import 'package:localpost/core/signal.dart';
import 'package:localpost/dialog/error_snackbar.dart';
import 'package:localpost/screen/home.dart';
import 'package:localpost/screen/transfer.dart';
import 'package:localpost/src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(const MyApp());

  await runServer(defaultPort);
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    scaffoldMessengerKey: scaffoldMessengerKey,
    debugShowCheckedModeBanner: false,
    themeMode: .dark,
    theme: .light(),
    darkTheme: .dark(),
    title: 'Local Post',
    home: const MainPage(),
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
    NavigationDestination(
      icon: Icon(Icons.swap_vert_outlined),
      selectedIcon: Icon(Icons.swap_vert),
      label: 'Transfer',
    ),
  ];

  @override
  Widget build(BuildContext context) => screenIdx.watch(
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
