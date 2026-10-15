import Flutter
import UIKit
import UserNotifications

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    // FlutterAppDelegate passes notification events on to PushBridge.
    UNUserNotificationCenter.current().delegate = self
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    if let registrar = engineBridge.pluginRegistry.registrar(forPlugin: "PushBridge") {
      PushBridge.register(with: registrar)
    }
  }
}

/// Asks for notification permission and hands Dart the APNs device token.
/// Channel `tightline.systems/push`:
///   `status`   -> "notDetermined" | "denied" | "authorized" | "provisional" | "ephemeral"
///   `register` -> the device token as lowercase hex, after asking permission if needed
final class PushBridge: NSObject, FlutterPlugin {
  private var waitingForToken: [FlutterResult] = []

  static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "tightline.systems/push", binaryMessenger: registrar.messenger())
    let bridge = PushBridge()
    registrar.addMethodCallDelegate(bridge, channel: channel)
    registrar.addApplicationDelegate(bridge)
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "status":
      UNUserNotificationCenter.current().getNotificationSettings { settings in
        DispatchQueue.main.async { result(Self.name(settings.authorizationStatus)) }
      }
    case "register":
      // iOS shows the permission prompt once and remembers the answer.
      UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) {
        granted, error in
        DispatchQueue.main.async {
          if let error = error {
            result(FlutterError(code: "permission", message: error.localizedDescription, details: nil))
          } else if !granted {
            result(FlutterError(code: "denied", message: "Notifications are off for Closing Tracker", details: nil))
          } else {
            self.waitingForToken.append(result)
            UIApplication.shared.registerForRemoteNotifications()
          }
        }
      }
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  func application(
    _ application: UIApplication,
    didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
  ) {
    let hex = deviceToken.map { String(format: "%02x", $0) }.joined()
    finish(hex)
  }

  func application(
    _ application: UIApplication,
    didFailToRegisterForRemoteNotificationsWithError error: Error
  ) {
    finish(FlutterError(code: "apns", message: error.localizedDescription, details: nil))
  }

  /// Shows alerts that arrive while the app is open, as iOS does when it's closed.
  func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    willPresent notification: UNNotification,
    withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
  ) {
    completionHandler([.banner, .list, .sound])
  }

  private func finish(_ value: Any) {
    let waiting = waitingForToken
    waitingForToken = []
    waiting.forEach { $0(value) }
  }

  private static func name(_ status: UNAuthorizationStatus) -> String {
    switch status {
    case .notDetermined: return "notDetermined"
    case .denied: return "denied"
    case .authorized: return "authorized"
    case .provisional: return "provisional"
    case .ephemeral: return "ephemeral"
    @unknown default: return "unknown"
    }
  }
}
