import 'package:flutter/material.dart';

class Lifecycle extends StatefulWidget {
  const Lifecycle({
    super.key,
    required this.builder,
    this.initState,
    this.dispose,
  });

  final Widget Function(BuildContext context) builder;
  final void Function()? initState;
  final void Function()? dispose;

  @override
  State<Lifecycle> createState() => _LifecycleState();
}

class _LifecycleState extends State<Lifecycle> {
  @override
  void initState() {
    super.initState();
    widget.initState?.call();
  }

  @override
  void dispose() {
    widget.dispose?.call();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return widget.builder(context);
  }
}
