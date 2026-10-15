# Closing Tracker

A free iPhone app from Tight Line that runs a shop's nightly closing checklist on the staff's phones and sends the owner one message: closing complete, what went wrong, or not done on time. Built to `Closing Tracker — Build Spec.md`; every choice and deviation is in `docs/DECISIONS.md`.

**Status: Phase 0, proving the pipeline** (`docs/PHASE0.md`). The app is a pipeline check that registers for alerts; the relay is a stub with health, a test registration and a test push.

```
app/       Flutter app (iOS now; Android kept but not built until Austin decides)
relay/     Rust relay: closing-relay (the service) and relayctl (operator tool)
ops/       relay.toml, systemd unit, Caddy site, setup-host.sh
docs/      DECISIONS.md, PHASE0.md (later: PROTOCOL.md, THREATS.md, STORE.md)
.github/   ci.yml (every push), ios.yml (TestFlight)
```

The website pieces (`/closing`, the `/c/<code>` card router, `/privacy#relay`, help) live in the website repo, `Aust1nian/tightline-systems`.

## Relay

```sh
cd relay
cargo test                     # unit and round-trip tests against a mock APNs
cargo zigbuild --release --target x86_64-unknown-linux-musl    # this workstation
cargo zigbuild --release --target aarch64-unknown-linux-musl   # the Pi
```

Zig and cargo-zigbuild are in `~/sdk/zig` and `~/.cargo/bin` on Austin's machine: `export PATH=~/sdk/zig:$PATH`.

On the relay host (members of `closing-relay-ops`):

```sh
relayctl status
relayctl push-tokens          # phones registered by the Phase 0 build
relayctl push-test [--to <first characters of a token>]
```

Install or update the host with `sudo bash ops/setup-host.sh` (see `--help`).

## App

```sh
export PATH=~/sdk/flutter/bin:$PATH   # Flutter 3.47.7
cd app
flutter test
flutter analyze
```

iOS builds only happen on GitHub's macOS runner: `ci.yml` makes an unsigned build on every push, and `ios.yml` signs and uploads to TestFlight once the App Store Connect secrets exist and the repository variable `TESTFLIGHT` is `on`.
