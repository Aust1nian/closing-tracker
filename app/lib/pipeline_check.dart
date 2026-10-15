import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'push.dart';
import 'relay.dart';

/// Set by CI (`--dart-define`); shown so a tester can say which build they ran.
const buildName = String.fromEnvironment('BUILD_NAME', defaultValue: 'dev');
const buildNumber = String.fromEnvironment('BUILD_NUMBER', defaultValue: '0');

/// TestFlight and App Store builds get production push tokens; Xcode runs get sandbox ones.
const apnsEnv = kReleaseMode ? 'production' : 'sandbox';

/// Phase 0: proves the relay is reachable and can push to this phone.
/// The gate: with the app swiped away and wifi off, the test alert arrives.
class PipelineCheckScreen extends StatefulWidget {
  const PipelineCheckScreen({
    super.key,
    required this.relay,
    required this.push,
  });

  final RelayApi relay;
  final PushApi push;

  @override
  State<PipelineCheckScreen> createState() => _PipelineCheckScreenState();
}

enum _Service { checking, connected, unreachable }

enum _Alerts { idle, working, registered, denied, unsupported, failed }

class _PipelineCheckScreenState extends State<PipelineCheckScreen> {
  _Service _service = _Service.checking;
  DateTime? _checkedAt;
  _Alerts _alerts = _Alerts.idle;
  String _tokenPrefix = '';
  String _failure = '';

  @override
  void initState() {
    super.initState();
    _checkService();
    widget.push.status().then((status) {
      if (!mounted) return;
      if (status == 'unsupported') {
        setState(() => _alerts = _Alerts.unsupported);
      } else if (status == 'denied') {
        setState(() => _alerts = _Alerts.denied);
      }
    });
  }

  Future<void> _checkService() async {
    setState(() => _service = _Service.checking);
    final ok = await widget.relay.healthy();
    if (!mounted) return;
    setState(() {
      _service = ok ? _Service.connected : _Service.unreachable;
      _checkedAt = DateTime.now();
    });
  }

  Future<void> _registerAlerts() async {
    setState(() => _alerts = _Alerts.working);
    try {
      final token = await widget.push.register();
      await widget.relay.registerPushCheck(
        token: token,
        env: apnsEnv,
        label: _label(),
      );
      if (!mounted) return;
      setState(() {
        _alerts = _Alerts.registered;
        _tokenPrefix = token.substring(0, 8);
      });
    } on PlatformException catch (e) {
      if (!mounted) return;
      setState(() {
        _alerts = e.code == 'denied' ? _Alerts.denied : _Alerts.failed;
        _failure = e.message ?? e.code;
      });
    } on Exception catch (e) {
      if (!mounted) return;
      setState(() {
        _alerts = _Alerts.failed;
        _failure = '$e';
      });
    }
  }

  String _label() {
    final version = RegExp(r'\d+(\.\d+)*')
        .firstMatch(Platform.operatingSystemVersion)
        ?.group(0);
    final os = Platform.isIOS ? 'iOS' : Platform.operatingSystem;
    return '$os ${version ?? ''} · build $buildNumber';
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Closing Tracker')),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            Text('Pipeline check', style: text.headlineSmall),
            const SizedBox(height: 4),
            Text('Build $buildName ($buildNumber)', style: text.bodySmall),
            const SizedBox(height: 16),
            _section(
              context,
              title: 'Service',
              body: _serviceText(context),
              action: _service == _Service.checking
                  ? null
                  : OutlinedButton(
                      onPressed: _checkService,
                      child: const Text('Check again'),
                    ),
            ),
            const SizedBox(height: 12),
            _section(
              context,
              title: 'Alerts',
              body: _alertsText(),
              action: switch (_alerts) {
                _Alerts.idle ||
                _Alerts.denied ||
                _Alerts.failed => FilledButton(
                  onPressed: _registerAlerts,
                  child: Text(
                    _alerts == _Alerts.idle
                        ? 'Allow notifications'
                        : 'Try again',
                  ),
                ),
                _ => null,
              },
            ),
            const SizedBox(height: 24),
            Text(
              'This build only checks that alerts can reach this phone. '
              'The closing checklist comes in later builds.',
              style: text.bodySmall,
            ),
          ],
        ),
      ),
    );
  }

  String _serviceText(BuildContext context) {
    final at = _checkedAt == null
        ? ''
        : ' Checked ${TimeOfDay.fromDateTime(_checkedAt!).format(context)}.';
    return switch (_service) {
      _Service.checking => 'Checking…',
      _Service.connected => 'Connected.$at',
      _Service.unreachable => "Can't reach the service.$at",
    };
  }

  String _alertsText() => switch (_alerts) {
    _Alerts.idle =>
      'Allow notifications so the test alert can reach this phone.',
    _Alerts.working => 'Registering this phone…',
    _Alerts.registered =>
      'This phone is registered as $_tokenPrefix. Now close the app '
          '(swipe it away), turn off wifi, and ask for the test alert.',
    _Alerts.denied =>
      'Notifications are off. Turn them on in Settings, Notifications, '
          'Closing Tracker, then tap Try again.',
    _Alerts.unsupported => 'Alerts on this phone come in a later build.',
    _Alerts.failed => "Couldn't register this phone: $_failure.",
  };

  Widget _section(
    BuildContext context, {
    required String title,
    required String body,
    Widget? action,
  }) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text(body),
            if (action != null) ...[const SizedBox(height: 12), action],
          ],
        ),
      ),
    );
  }
}
