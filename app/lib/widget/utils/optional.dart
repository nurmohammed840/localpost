import 'package:flutter/material.dart';

class Optional extends StatelessWidget {
  const Optional({super.key, required this.condition, required this.builder});

  final bool condition;
  final WidgetBuilder builder;

  @override
  Widget build(BuildContext context) {
    if (condition) return builder(context);
    return const SizedBox.shrink();
  }
}

extension NullableWidget<T> on T? {
  Widget mapWidget(Widget Function(BuildContext context, T value) builder) {
    if (this == null) return const SizedBox.shrink();
    return Builder(
      builder: (context) {
        return builder(context, this as T);
      },
    );
  }
}
