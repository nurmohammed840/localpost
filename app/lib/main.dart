import 'package:flutter/material.dart';
import 'package:localpost/settings.dart';
import 'package:localpost/signal.dart';
import 'package:localpost/src/rust/api/bridge.dart';
import 'package:localpost/src/rust/frb_generated.dart';
import 'package:localpost/utils.dart';

Future<void> main() async {
  await RustLib.init();
  final addr = getLocalIp();
  runApp(MyApp(addr: addr));
}

class MyApp extends StatelessWidget {
  const MyApp({super.key, required this.addr});

  final String addr;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      themeMode: .dark,
      theme: .light(),
      darkTheme: .dark(),
      title: 'LocalPost',
      debugShowCheckedModeBanner: false,
      home: HomePage(addr: addr),
    );
  }
}

class HomePage extends StatelessWidget {
  HomePage({super.key, required this.addr});

  final String addr;

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
      label: 'Transfers',
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

class HomeScreen extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('LocalPost'),
        actionsPadding: const .only(right: 10),
        actions: [
          IconButton(
            tooltip: 'Settings',
            icon: const Icon(Icons.settings_outlined),
            onPressed: () {
              Navigator.push(
                context,
                MaterialPageRoute(builder: (_) => const Settings()),
              );
            },
          ),
        ],
      ),
    );
  }
}

class TransferScreen extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Transfer'),
        actionsPadding: const .only(right: 10),
      ),
    );
  }
}

Future<String?> promptIPAddr(BuildContext context) async {
  final controller = TextEditingController(text: '192.168.0.1');

  return showDialog<String>(
    context: context,
    builder: (_) => AlertDialog(
      title: const Text('Connect Device'),
      content: TextField(
        controller: controller,
        autofocus: true,
        keyboardType: .number,
        inputFormatters: [IPv4Formatter()],
        decoration: const InputDecoration(labelText: 'IP address'),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, controller.text),
          child: const Text('Connect'),
        ),
      ],
    ),
  );
}
