import 'package:flutter/material.dart';

class Scoped extends StatefulWidget {
  const Scoped({
    super.key,
    required this.builder,
    this.initState,
    this.dispose,
  });

  final Widget Function(BuildContext context) builder;
  final void Function(BuildContext context)? initState;
  final void Function(BuildContext context)? dispose;

  @override
  State<Scoped> createState() => _ScopedState();
}

class _ScopedState extends State<Scoped> {
  @override
  void initState() {
    super.initState();
    widget.initState?.call(context);
  }

  @override
  void dispose() {
    widget.dispose?.call(context);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return widget.builder(context);
  }
}
