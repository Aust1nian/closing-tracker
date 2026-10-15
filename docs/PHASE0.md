# Phase 0: prove the pipeline

**Gate:** a push sent from the relay host reaches Austin's iPhone over 5G with the app force-closed.

What's built and tested on the workstation is listed in the README. The steps below need Austin's accounts, sudo or phone, in this order. Each says what to send back.

## 1. App Store Connect API key (5 min)

App Store Connect, **Users and Access**, **Integrations**, **App Store Connect API**, **Team Keys**, **+**:
- Name: `Closing Tracker CI`. Access: **Admin** (Xcode's cloud signing needs it).
- Download the `.p8` (Apple allows one download). Note the **Key ID** and the **Issuer ID** at the top of the page.

Then put it in this repo's GitHub secrets from the workstation (the key never goes into the repo):

```sh
cd "/mnt/disk1/Burnes_Hink_&_Co/Attraction_Phase/Gifts/Closing_Tracker"
gh secret set ASC_KEY_ID --body <KEY ID>
gh secret set ASC_ISSUER_ID --body <ISSUER ID>
base64 -w0 ~/Downloads/AuthKey_<KEY ID>.p8 | gh secret set ASC_KEY_P8
gh secret set APPLE_TEAM_ID --body J85MUL5KVY
```

Keep the `.p8` somewhere safe, such as a password manager, then delete the download.

## 2. App ID and App Store Connect record (5 min)

1. developer.apple.com, **Certificates, Identifiers & Profiles**, **Identifiers**, **+**, **App IDs**, **App**:
   - Description `Closing Tracker`, Bundle ID **Explicit** `tightline.systems.closingtracker`.
   - Capabilities: tick **Push Notifications**. Register.
2. App Store Connect, **Apps**, **+**, **New App**: iOS, name `Closing Tracker`, language English (U.S.), bundle ID `tightline.systems.closingtracker`, SKU `closing-tracker`, Full Access. This reserves the name.
3. In the new app, **TestFlight**, **Internal Testing**, **+**: a group named `Austin` with yourself in it.

Send back: "record created". Then I turn on the TestFlight workflow (`gh variable set TESTFLIGHT --body on`) and run it.

## 3. APNs key for the relay (3 min)

developer.apple.com, **Keys**, **+**: name `Closing Tracker relay`, tick **Apple Push Notifications service (APNs)**, environment **Sandbox & Production**, then Register. Download `AuthKey_<KEY ID>.p8`. Again, one download only, so keep a safe copy.

## 4. DNS (2 min)

Network Solutions, `tightline.systems`, DNS: add an **A** record `relay` pointing to the house's public address, the same address as the `tightline.systems` A record.

## 5. Install the relay on the workstation (sudo, 2 min)

```sh
cd "/mnt/disk1/Burnes_Hink_&_Co/Attraction_Phase/Gifts/Closing_Tracker"
sudo bash ops/setup-host.sh --caddy --apns-key ~/Downloads/AuthKey_<KEY ID>.p8
```

This creates the `closing-relay` user, installs the binaries, config, key and systemd service, adds the `relay.tightline.systems` site to Caddy (it validates first and puts things back if Caddy refuses), and checks the relay answers. Once DNS has spread:

```sh
curl https://relay.tightline.systems/v1/health
```

## 6. The gate test (5 min)

1. Install the build from TestFlight on the iPhone and open it. **Service** should say Connected.
2. Tap **Allow notifications** and allow. It says "This phone is registered as `xxxxxxxx`".
3. Swipe the app away. Turn wifi off so the phone is on 5G.
4. Tell me it's ready. I send `relayctl push-test` from the workstation.
5. "Test alert: you're set up." appears on the lock screen.
