import 'package:flutter/material.dart';

import 'pipeline_check.dart';
import 'push.dart';
import 'relay.dart';

void main() => runApp(ClosingTrackerApp(relay: Relay(), push: Push()));

class ClosingTrackerApp extends StatelessWidget {
  const ClosingTrackerApp({super.key, required this.relay, required this.push});

  final RelayApi relay;
  final PushApi push;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Closing Tracker',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF1F2A30)),
      ),
      darkTheme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF1F2A30),
          brightness: Brightness.dark,
        ),
      ),
      home: PipelineCheckScreen(relay: relay, push: push),
    );
  }
}
