import 'dart:async';
import 'dart:convert';
import 'dart:io';

/// The app knows the relay only by its domain name, never by IP, so moving
/// the relay to another machine is a DNS change, not an app update.
const relayOrigin = String.fromEnvironment(
  'RELAY_ORIGIN',
  defaultValue: 'https://relay.tightline.systems',
);

abstract interface class RelayApi {
  /// True when `/v1/health` answers `ok`.
  Future<bool> healthy();

  /// Phase 0 only: lets the operator send this phone a test alert.
  Future<void> registerPushCheck({
    required String token,
    required String env,
    required String label,
  });
}

class Relay implements RelayApi {
  Relay({HttpClient? client, this.origin = relayOrigin})
    : _client =
          client ??
          (HttpClient()..connectionTimeout = const Duration(seconds: 10));

  final HttpClient _client;
  final String origin;

  @override
  Future<bool> healthy() async {
    try {
      final req = await _client.getUrl(Uri.parse('$origin/v1/health'));
      final resp = await req.close().timeout(const Duration(seconds: 10));
      final body = await resp.transform(utf8.decoder).join();
      if (resp.statusCode != 200) return false;
      final json = jsonDecode(body);
      return json is Map && json['ok'] == true;
    } on Exception {
      return false;
    }
  }

  @override
  Future<void> registerPushCheck({
    required String token,
    required String env,
    required String label,
  }) async {
    final req = await _client.putUrl(Uri.parse('$origin/v0/push-check'));
    req.headers.contentType = ContentType.json;
    req.write(jsonEncode({'token': token, 'env': env, 'label': label}));
    final resp = await req.close().timeout(const Duration(seconds: 15));
    await resp.drain<void>();
    if (resp.statusCode != HttpStatus.noContent) {
      throw RelayException(resp.statusCode);
    }
  }
}

class RelayException implements Exception {
  RelayException(this.status);

  final int status;

  @override
  String toString() => status == HttpStatus.tooManyRequests
      ? 'too many tries from this network; wait an hour'
      : 'the service answered HTTP $status';
}
