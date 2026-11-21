import 'package:flutter/material.dart';

class Lifecycle<T> extends StatefulWidget {
  const new({super.key, this.initState, this.dispose, required this.builder});

  final T Function()? initState;
  final void Function(T value)? dispose;

  final Widget Function(BuildContext context, T value) builder;

  @override
  State<Lifecycle<T>> createState() => _LifecycleState<T>();
}

class _LifecycleState<T> extends State<Lifecycle<T>> {
  T? _value;

  @override
  void initState() {
    super.initState();
    _value = widget.initState?.call();
  }

  @override
  void dispose() {
    widget.dispose?.call(_value as T);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.builder(context, _value as T);
}
