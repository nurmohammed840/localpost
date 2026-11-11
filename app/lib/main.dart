import 'package:flutter/material.dart';
import 'package:localpost/screen/home.dart';
import 'package:localpost/screen/transfer.dart';
import 'package:localpost/signal.dart';
import 'package:localpost/src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
  // final addr = Server.bind();
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      themeMode: .dark,
      theme: .light(),
      darkTheme: .dark(),
      title: 'Local Post',
      debugShowCheckedModeBanner: false,
      home: HomePage(),
    );
  }
}

class HomePage extends StatelessWidget {
  HomePage({super.key});

  final selectedNav = CreateState(0);

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

  static const screens = [HomeScreen(), TransferScreen()];

  @override
  Widget build(BuildContext context) {
    return selectedNav.watch(
      (_) => Scaffold(
        body: IndexedStack(index: selectedNav.value, children: screens),
        bottomNavigationBar: NavigationBar(
          labelBehavior: .alwaysHide,
          selectedIndex: selectedNav.value,
          onDestinationSelected: selectedNav.set,
          destinations: destinations,
        ),
      ),
    );
  }
}
