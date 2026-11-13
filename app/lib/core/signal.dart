import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

class CreateState<T> extends ChangeNotifier implements ValueListenable<T> {
  /// Creates a [ChangeNotifier] that wraps this value.
  CreateState(this._value);

  /// The current value stored in this notifier.
  ///
  /// When the value is replaced with something that is not equal to the old
  /// value as evaluated by the equality operator ==, this class notifies its
  /// listeners.
  @override
  T get value => _value;
  T _value;

  set value(T newValue) {
    _value = newValue;
  }

  void update(void Function(T) cb) {
    cb(_value);
    notifyListeners();
  }

  void set(T value) {
    if (_value == value) {
      return;
    }

    _value = value;
    notifyListeners();
  }

  void notify() {
    notifyListeners();
  }

  Watch watch(WatchableWidgetBuilder builder) {
    return Watch(signal: this, builder: builder);
  }

  @override
  String toString() => '${describeIdentity(this)}($value)';
}

typedef WatchableWidgetBuilder = Widget Function(BuildContext context);

class Watch extends StatefulWidget {
  const Watch({super.key, required this.signal, required this.builder});

  final Listenable signal;
  final WatchableWidgetBuilder builder;

  @override
  State<Watch> createState() => _WatchState();
}

class _WatchState extends State<Watch> {
  @override
  void initState() {
    super.initState();
    widget.signal.addListener(_setState);
  }

  @override
  void didUpdateWidget(Watch old) {
    super.didUpdateWidget(old);
    if (old.signal != widget.signal) {
      old.signal.removeListener(_setState);
      widget.signal.addListener(_setState);
    }
  }

  @override
  void dispose() {
    widget.signal.removeListener(_setState);
    super.dispose();
  }

  void _setState() {
    if (mounted) {
      setState(() {});
    }
  }

  @override
  Widget build(BuildContext context) {
    return widget.builder(context);
  }
}
