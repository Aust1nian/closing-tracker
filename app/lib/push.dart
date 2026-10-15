import 'package:flutter/services.dart';

/// Notification permission and the device's push token.
abstract interface class PushApi {
  /// iOS's permission state, `unsupported` where the platform side isn't built yet.
  Future<String> status();

  /// Asks permission if needed and returns the APNs device token as hex.
  Future<String> register();
}

/// The iOS side lives in `ios/Runner/AppDelegate.swift` (PushBridge).
class Push implements PushApi {
  static const _channel = MethodChannel('tightline.systems/push');

  @override
  Future<String> status() async {
    try {
      return await _channel.invokeMethod<String>('status') ?? 'unknown';
    } on MissingPluginException {
      return 'unsupported';
    }
  }

  @override
  Future<String> register() async {
    final token = await _channel.invokeMethod<String>('register');
    if (token == null || token.isEmpty) {
      throw PlatformException(code: 'apns', message: 'Apple sent no token');
    }
    return token;
  }
}
