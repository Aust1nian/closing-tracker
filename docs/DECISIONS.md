# Decisions

Every choice the spec (`Closing Tracker — Build Spec.md`) leaves open, and every deviation from it, with the date and the reason. Newest entries go at the bottom of each section.

## Austin's answers to the open questions

**2026-10-15 · Q1, iOS builds: Flutter, fastlane and GitHub's macOS runner, in a public repo.** The spec default. Earshot's TestFlight workflow (`/mnt/disk1/Transcription_App/.github/workflows/ios.yml`) was never run, because that repo has no GitHub remote, so there was no proven pipeline to reuse. Its approach (Xcode automatic signing through an App Store Connect API key) is what `app/ios/fastlane/Fastfile` uses. The repo is public, like Schedule Spotter, so GitHub's macOS minutes cost nothing (hard constraint 1). Flutter 3.47.7 (stable on 2026-10-08) is pinned in both workflows; iOS builds run on `macos-26` with the newest stable Xcode, because App Store uploads need the iOS 26 SDK.

**2026-10-15 · Q2, relay host: The-Money-Engine for now, then a Raspberry Pi 4B (Austin's call; the spec forbids The-Money-Engine).** This is an exception to hard constraint 11 and to the "hands off The-Money-Engine" rule, which Austin approved after being shown the conflict. It also differs from the website's 2026-10-11 note that gift software never runs on the workstation. How the exception is contained:
- The relay runs as its own system user (`closing-relay`) under a sandboxed systemd unit (`ops/closing-relay.service`). It sees only its state directory, its config and the network. `/mnt`, `/media`, `/srv`, `/opt` and every home directory are invisible to it, it holds no capabilities, and it listens on loopback only.
- The website's Caddy, which already owns port 443 at the house, terminates TLS for `relay.tightline.systems` and forwards to `127.0.0.1:8741` (`ops/caddy/closing-relay.caddy`). There's no new port forward and no new certificate setup.
- Moving to the Pi means installing the aarch64 binary and the same files there with `ops/setup-host.sh`, then changing the one `reverse_proxy` line to the Pi's LAN address (or pointing DNS at it). The apps don't change, because they know the relay only by its domain name.

Cloudflare Tunnel, the spec's default, is out. Austin took Cloudflare out of the website on 2026-10-11, and a tunnel on `relay.tightline.systems` would need the domain's DNS moved from Network Solutions to Cloudflare.

**2026-10-15 · Q2, domain: `relay.tightline.systems`.** The website already names it (its CSP `connect-src` and the `/status` page's health check).

**2026-10-15 · Q3, Google Play dropped; Android later (Austin's call).** v1 is iPhone-first. The Flutter project keeps its Android side (application ID `tightline.systems.closingtracker`, minimum Android 8.0, API 26) so Android can return later, but nothing Android is built, tested or shipped until Austin decides. What changes:
- **Devices** (a settled decision): owner and staff need iPhones for now.
- **Push:** the relay sends straight to Apple (APNs, token auth with a `.p8` key), the spec's listed alternative. No Firebase project and no Google account. FCM comes back only with Android.
- **Phase 0 gate:** a push from the relay host reaches Austin's iPhone over 5G with the app force-closed. The Android half, Play internal testing and the 12-tester closed test are gone.
- **Acceptance tests:** "iPhone staff to Android owner, and the reverse" becomes iPhone to iPhone until Android returns.
- **Website (flag for that repo):** the `/c` card router, `/closing` and the `/support` copy mention Google Play. With no Play URL in `stores.json` the router already sends Android phones nowhere, but the words should say iPhone only.

**2026-10-15 · Q4, app identity.** Name "Closing Tracker" (not taken in a US App Store search on 2026-10-15; reserved only when the App Store Connect record is created). Bundle ID `tightline.systems.closingtracker`, exactly as Austin wrote it. It can never change after the first upload. The icon is a draft for now: the Tight Line mark on white (`app/tool/render_icon.mjs`, `app/tool/flatten_icon.py`). Austin approves the final icon before store submission.

## Build choices

**2026-10-15 · Relay HTTPS client: reqwest 0.13 with rustls on `ring`, not aws-lc.** `ring` cross-compiles cleanly to static musl binaries with zig. The APNs provider token (ES256 JWT) is signed by `jsonwebtoken` 11 with its pure-Rust backend. A live check shows the client reaches `api.push.apple.com` over HTTP/2, which APNs requires (`real_apns_speaks_http2`, an ignored test that talks to Apple).

**2026-10-15 · Static binaries with cargo-zigbuild.** `cargo zigbuild --release --target {x86_64,aarch64}-unknown-linux-musl` gives one statically linked `closing-relay` and `relayctl` per architecture (hard constraint 8). CI builds and checks both on every push. Zig lives in `~/sdk/zig` on Austin's machine and comes from pip in CI.

**2026-10-15 · `relayctl` talks to the running relay over a Unix socket** (`/run/closing-relay/admin.sock`, mode 0660, group `closing-relay-ops`), not by opening the database. The APNs key and database stay readable only by the relay's user, and operators (Austin, via the group) need no sudo to check status or send a test push.

**2026-10-15 · Phase 0 registration endpoint: `PUT /v0/push-check`.** It's unauthenticated because there are no device tokens until Phase 1, so it's kept small: 10 registrations per address per hour (IPv6 counted per /64), only the newest 50 tokens kept, and nothing but the APNs token, its environment and a short label stored. Phase 1 removes it along with its table, replacing it with the authenticated `PUT /v1/push-token`.

**2026-10-15 · Relay database: SQLite in WAL mode with `synchronous=FULL`.** An acknowledged write survives a power cut (acceptance test "power pulled on the Pi"). The schema is versioned by `user_version`.

**2026-10-15 · Health check shape.** `GET /v1/health` answers `{"ok":true,…}` with `Access-Control-Allow-Origin: https://tightline.systems` and `Cache-Control: no-store`, which is what the website's `/status` page reads.

**2026-10-15 · No `site/` folder in this repo (deviation from the spec's layout).** The website repo (`Aust1nian/tightline-systems`) already has `/closing`, the `/c/<code>` card router, `/support#closing` and the one privacy policy every free tool shares (`/privacy#relay`, Austin's 2026-10-12 decision). The store listings will use `https://tightline.systems/privacy`.

**2026-10-15 · iOS app scaffolding.** Minimum iOS 16 (spec). Flutter 3.47's template uses the UIScene lifecycle, so the push code (`PushBridge`) lives in `ios/Runner/AppDelegate.swift` and registers from `didInitializeImplicitFlutterEngine`. It isn't a plugin package and needs no Xcode project edits beyond the entitlements file. `aps-environment` is `development` in the file; Xcode signs TestFlight and App Store builds with `production`.

**2026-10-15 · Export compliance: `ITSAppUsesNonExemptEncryption = false` for the Phase 0 build only.** This build uses nothing but the system's HTTPS. Once libsodium end-to-end encryption arrives (Phase 2), the answer changes and has to be worked out for App Store Connect before that build is uploaded (`docs/STORE.md`, Phase 7).

**2026-10-15 · App Store Connect API key for CI: a Team key with the Admin role.** Xcode's cloud-managed distribution signing through an API key needs Admin. Schedule Spotter's key has the Developer role and its `.p8` can't be read back from GitHub anyway, so Closing Tracker gets its own key, stored only in this repo's Actions secrets.

## Conflicts to settle when their phase comes

- **Phase 4, join alert text.** The spec's "Maria's phone wants to join Main St." puts a staff name in a push (Apple sees it) and on the relay. The spec's own privacy table and the website's `/privacy` say names never reach either. The default will be "A new phone wants to join Main St.", with the name shown inside the app only. To be confirmed with Austin in Phase 4.
- **Phase 1, retention the privacy policy doesn't state yet:** gift code redemption records, the addresses kept for rate limits, the nightly relay backups, and whether Delete shop also clears backups.
