import 'package:flutter/material.dart';

class Optional extends StatelessWidget {
  const Optional({super.key, required this.condition, required this.builder});

  final bool condition;
  final WidgetBuilder builder;

  @override
  Widget build(ctx) {
    if (condition) return builder(ctx);
    return const SizedBox.shrink();
  }
}

extension NullableWidget<T> on T? {
  Widget mapWidget(Widget Function(T value) builder) {
    final value = this;
    if (value == null) return const SizedBox.shrink();
    return builder(value);
  }
}
