#!/usr/bin/env bash
# Installs or updates the Closing Tracker relay on this host: a system user, the
# static binaries, /etc/closing-relay, the systemd service and, with --caddy, the
# Caddy site for relay.tightline.systems. Safe to run again: it never overwrites
# relay.toml, and replaces the push key only when given one.
#
#   sudo bash ops/setup-host.sh --caddy                               first install, pushes off
#   sudo bash ops/setup-host.sh --apns-key ~/Downloads/AuthKey_ABCDE12345.p8
#                                                                     add or replace the push key
#   sudo bash ops/setup-host.sh                                       update the binaries
#
# Options:
#   --apns-key FILE   the .p8 from developer.apple.com, Keys. The key ID is read
#                     from its name (AuthKey_<KEY ID>.p8).
#   --apns-key-id ID  the key ID, if the file was renamed
#   --caddy           also install the Caddy site (on the machine whose Caddy owns port 443)
#   --bin DIR         where the built binaries are
#                     (default: relay/target/<this machine's arch>-unknown-linux-musl/release)
set -euo pipefail

say() { printf '\n== %s\n' "$*"; }
die() {
  printf 'setup-host: %s\n' "$*" >&2
  exit 1
}

[[ $EUID -eq 0 ]] || die "run it with sudo: sudo bash $0 $*"
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ARCH=$(uname -m)
BIN="$REPO/relay/target/$ARCH-unknown-linux-musl/release"
APNS_KEY=
APNS_KEY_ID=
CADDY=0
while [[ $# -gt 0 ]]; do
  case $1 in
    --apns-key) APNS_KEY=$2; shift 2 ;;
    --apns-key-id) APNS_KEY_ID=$2; shift 2 ;;
    --caddy) CADDY=1; shift ;;
    --bin) BIN=$2; shift 2 ;;
    -h | --help) sed -n '2,20p' "$0"; exit 0 ;;
    *) die "unknown option $1 (try --help)" ;;
  esac
done

for b in closing-relay relayctl; do
  [[ -x $BIN/$b ]] || die "$BIN/$b isn't built yet: cd relay && cargo zigbuild --release --target $ARCH-unknown-linux-musl"
done
if [[ -n $APNS_KEY ]]; then
  [[ -r $APNS_KEY ]] || die "can't read $APNS_KEY"
  grep -q 'BEGIN PRIVATE KEY' "$APNS_KEY" || die "$APNS_KEY doesn't look like a .p8 key"
  if [[ -z $APNS_KEY_ID ]]; then
    APNS_KEY_ID=$(basename "$APNS_KEY" | sed -n 's/^AuthKey_\([A-Z0-9]\{10\}\)\.p8$/\1/p')
    [[ -n $APNS_KEY_ID ]] || die "can't tell the key ID from the name $(basename "$APNS_KEY"); add --apns-key-id"
  fi
  [[ $APNS_KEY_ID =~ ^[A-Z0-9]{10}$ ]] || die "an APNs key ID is 10 capital letters and digits, not $APNS_KEY_ID"
fi
CONF=/etc/closing-relay/relay.toml

say "User and groups"
getent group closing-relay-ops >/dev/null || groupadd --system closing-relay-ops
id closing-relay >/dev/null 2>&1 ||
  useradd --system --user-group --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin closing-relay
if [[ -n ${SUDO_USER:-} && $SUDO_USER != root ]]; then
  usermod -aG closing-relay-ops "$SUDO_USER"
  echo "$SUDO_USER may use relayctl (in a new login, or now with: sg closing-relay-ops -c 'relayctl status')"
fi

say "Binaries"
install -m 0755 -o root -g root "$BIN/closing-relay" "$BIN/relayctl" /usr/local/bin/
/usr/local/bin/closing-relay --version

say "Config"
install -d -m 0755 -o root -g root /etc/closing-relay
apns_section() { sed -n '/^# Apple Push Notification service/,$p' "$REPO/ops/relay.toml"; }
if [[ ! -e $CONF ]]; then
  # Without a key the [apns] section stays out, so the relay starts with pushes off.
  sed '/^# Apple Push Notification service/,$d' "$REPO/ops/relay.toml" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' >"$CONF"
  chmod 0644 "$CONF"
  echo "wrote $CONF"
else
  echo "kept $CONF"
fi
if [[ -n $APNS_KEY ]]; then
  if grep -q '^\[apns\]' "$CONF"; then
    sed -i "s/^key_id = .*/key_id = \"$APNS_KEY_ID\"/" "$CONF"
  else
    { echo; apns_section | sed "s/^key_id = .*/key_id = \"$APNS_KEY_ID\"/"; } >>"$CONF"
  fi
  install -d -m 0700 -o closing-relay -g closing-relay /etc/closing-relay/apns
  install -m 0400 -o closing-relay -g closing-relay "$APNS_KEY" /etc/closing-relay/apns/AuthKey.p8
  echo "installed APNs key $APNS_KEY_ID (readable only by closing-relay)"
fi
runuser -u closing-relay -- /usr/local/bin/closing-relay check-config --config "$CONF"

say "Service"
install -m 0644 -o root -g root "$REPO/ops/closing-relay.service" /etc/systemd/system/closing-relay.service
systemctl daemon-reload
systemctl enable closing-relay >/dev/null
systemctl restart closing-relay
for _ in $(seq 20); do
  curl -fsS http://127.0.0.1:8741/v1/health >/dev/null 2>&1 && break
  sleep 0.5
done
if ! curl -fsS http://127.0.0.1:8741/v1/health; then
  journalctl -u closing-relay -n 30 --no-pager
  die "the relay didn't answer on 127.0.0.1:8741"
fi
echo

if [[ $CADDY == 1 ]]; then
  say "Caddy: relay.tightline.systems"
  command -v caddy >/dev/null || die "caddy isn't installed"
  install -d -m 0755 /etc/caddy/sites.d
  install -m 0644 "$REPO/ops/caddy/closing-relay.caddy" /etc/caddy/sites.d/closing-relay.caddy
  if ! grep -q '^import /etc/caddy/sites.d/\*.caddy' /etc/caddy/Caddyfile; then
    cp -p /etc/caddy/Caddyfile /etc/caddy/Caddyfile.before-relay
    printf '\n# Other services on this host, one file each (the Closing Tracker relay).\nimport /etc/caddy/sites.d/*.caddy\n' \
      >>/etc/caddy/Caddyfile
    echo "added the sites.d import to /etc/caddy/Caddyfile (the old one is Caddyfile.before-relay)"
  fi
  log=$(mktemp)
  if caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile >"$log" 2>&1; then
    systemctl reload caddy
    echo "Caddy reloaded"
  else
    grep -v '"level":"info"' "$log" || true
    [[ -e /etc/caddy/Caddyfile.before-relay ]] && cp -p /etc/caddy/Caddyfile.before-relay /etc/caddy/Caddyfile
    rm -f /etc/caddy/sites.d/closing-relay.caddy
    die "Caddy rejected the new config, so it's back as it was"
  fi
  rm -f "$log"
fi

say "Done"
systemctl --no-pager --lines=0 status closing-relay | head -3
cat <<EOF
Check it any time:
  sg closing-relay-ops -c 'relayctl status'
  curl https://relay.tightline.systems/v1/health     (once the DNS record is in)
EOF
