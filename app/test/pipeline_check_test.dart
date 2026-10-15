import 'package:closing_tracker/main.dart';
import 'package:closing_tracker/push.dart';
import 'package:closing_tracker/relay.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

class FakeRelay implements RelayApi {
  FakeRelay({this.up = true});

  bool up;
  final registered = <Map<String, String>>[];

  @override
  Future<bool> healthy() async => up;

  @override
  Future<void> registerPushCheck({
    required String token,
    required String env,
    required String label,
  }) async {
    registered.add({'token': token, 'env': env, 'label': label});
  }
}

class FakePush implements PushApi {
  FakePush({this.permission = 'notDetermined', this.deny = false});

  final String permission;
  final bool deny;

  @override
  Future<String> status() async => permission;

  @override
  Future<String> register() async {
    if (deny) {
      throw PlatformException(code: 'denied', message: 'Notifications are off');
    }
    return 'abababab${'00' * 28}';
  }
}

void main() {
  testWidgets('connected relay, then registering alerts', (tester) async {
    final relay = FakeRelay();
    await tester.pumpWidget(ClosingTrackerApp(relay: relay, push: FakePush()));
    await tester.pumpAndSettle();

    expect(find.textContaining('Connected.'), findsOneWidget);
    await tester.tap(find.text('Allow notifications'));
    await tester.pumpAndSettle();

    expect(find.textContaining('registered as abababab'), findsOneWidget);
    expect(relay.registered, hasLength(1));
    expect(
      relay.registered.single['env'],
      'sandbox',
      reason: 'tests run in debug mode',
    );
    expect(find.text('Allow notifications'), findsNothing);
  });

  testWidgets('unreachable relay and denied permission', (tester) async {
    final relay = FakeRelay(up: false);
    await tester.pumpWidget(
      ClosingTrackerApp(relay: relay, push: FakePush(deny: true)),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining("Can't reach the service."), findsOneWidget);
    await tester.tap(find.text('Allow notifications'));
    await tester.pumpAndSettle();

    expect(find.textContaining('Notifications are off.'), findsOneWidget);
    expect(find.text('Try again'), findsOneWidget);
    expect(relay.registered, isEmpty);

    relay.up = true;
    await tester.tap(find.text('Check again'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Connected.'), findsOneWidget);
  });

  testWidgets('a platform without the push side says so', (tester) async {
    await tester.pumpWidget(
      ClosingTrackerApp(
        relay: FakeRelay(),
        push: FakePush(permission: 'unsupported'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('come in a later build'), findsOneWidget);
    expect(find.text('Allow notifications'), findsNothing);
  });
}
