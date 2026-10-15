# Closing Tracker — Build Spec

Oct 10, 2026 · @Austin Burnes

## What it is

Closing Tracker is a free phone app (iPhone and Android) that runs a shop's nightly closing checklist on the staff's phones and sends the owner one message: closing complete, what went wrong, or not done on time.

- **The chore it removes:** the owner calling, texting or driving back to check the place got closed right.
- **What it builds toward a sale:** a dated record of every closing (on time, late, missed, issues), exportable as a monthly log. It shows a buyer the business runs without the owner watching.
- **Who uses it:** the owner, in admin mode, unlocked with the gift card's code; the closing staff, in staff mode, joined by scanning a code on the owner's phone.
- **Who it's for:** owners of $1–20M businesses building toward an eventual sale, in any business with a nightly close: restaurants, retail, salons and spas, contractor offices and yards.
- **How it differs from the three desktop gifts:** it's a phone app from the App Store and Google Play, and it needs one small always-on relay, hosted by Austin, to carry messages between phones on different networks.

## Settled decisions

These were decided with Austin in planning and are not open for Claude Code to change without asking.

| Area | Decision |
| --- | --- |
| Distribution | One app on the Apple App Store and Google Play, under Austin's existing Apple Developer account and a Google Play account ($25, one time). |
| Modes | One app, two modes: admin mode for the owner, unlocked by the gift card's code; staff mode for everyone else. |
| Devices | Staff can be on iPhone or Android. Either one can take photos, complete the checklist and finish the closing. |
| Connectivity | Must work over any network, 5G or wifi. Never assume the owner is on the shop's wifi. |
| Relay | One always-on relay passes messages between phones. It runs on Austin's own hardware, dedicated to this job. No monthly subscription. |
| Scale | One relay serves every shop that gets the gift. |
| Privacy | The relay stores nothing it can read. Checklists, names, notes and photos are end-to-end encrypted, and the relay deletes each message once it's delivered. |
| Offline | Every tick, photo and finished closing is saved on the phone first. If the relay is off, everything waits in an outbox, sends automatically when it's back, and keeps its original time. Nothing is lost. |
| Owner messages | "Closing complete, no issues"; complete with issues; "staff did not complete closing on time." |
| Build | Built with Claude Code on Austin's Linux machine. Claude Code asks Austin about anything unresolved. |

## Architecture

Staff and owner phones never connect to each other: each sends sealed messages out to the relay, which forwards them and triggers alerts through Firebase's free push service.

&#91;embedded content: architecture · phones, relay on the Pi, push services\]

The relay is the only part on Austin's hardware, and everything readable lives on the phones.

## How it's used

Owner setup takes under 5 minutes with the salesperson present; each staff phone joins in under a minute; after that the nightly close runs without the owner.

### Owner setup

1. The owner scans the gift card's QR with the phone camera. It opens a page on the company site that sends iPhones to the App Store and Android phones to Google Play.
2. In the app: **Set up my shop**, then scan the same card with the in-app scanner, or type its 8-character code.
3. Name the shop (the label used in alerts) and confirm the time zone, which comes from the phone.
4. Pick a starter checklist for their trade and edit it: add, remove, reorder, mark items as photo-required, give number items an allowed range.
5. Set the schedule: days open, the time closing must be done by for each day, and the grace period.
6. Allow notifications, then tap **Send me a test alert.** Setup isn't finished until the test alert arrives.
7. Add staff (below). Recommend adding a second admin device too, such as the owner's tablet or a manager.

### Staff joining

1. The staff member installs the app.
2. The owner opens **Team → Add staff**, which shows a join QR that expires in 15 minutes, or taps **Share join link** for someone not in the room.
3. The staff member scans or taps it and types a first name.
4. The owner's phone shows "Maria's phone wants to join. Approve?" The owner approves.
5. Tonight's checklist appears on the staff phone. No account, email or phone number is collected.

### The nightly close

1. Tonight's checklist shows on every staff phone from the opening time, by default 3 hours before the deadline.
2. Anyone on the crew ticks items. Ticks sync to the other staff phones and the owner within seconds when online.
3. Photo items open the in-app camera; the photo gallery is never offered. Number items take a value, and a value outside the range is an issue. Any item can be marked **Can't do this** with a reason, which is also an issue.
4. **Finish closing** unlocks once every required item is done or marked can't-do. A summary screen, then submit. The closing is now locked.
5. The owner gets one message. Tapping it opens the report: who did what and when, the photos, and any issues.

### Where it fits the sales sequence

Building the owner's checklist is the discovery interview: "What gets missed at close? What did it cost last time?" The app downloads in seconds, so this replaces the download-wait interview the desktop tools used. The **This month** screen (on time, late, missed, issues) is the agenda for the 15-minute check-in.

## Messages the owner gets

The owner gets exactly one message per closing in the normal case, plus a correction if a late report changes the picture. Times shown are examples. When the owner has more than one shop, every message starts with the shop's label.

| Situation | Push text | Sent when |
| --- | --- | --- |
| Finished on time, no issues | Closing complete, 9:48 pm. No issues. | Within 60 s of Finish |
| Finished on time, with issues | Closing complete, 9:48 pm. 2 issues, tap to see. | Within 60 s of Finish |
| Finished after the deadline | Closing finished late: 11:05 pm (due 10:30). No issues. | Within 60 s of Finish |
| Started, not finished by deadline + grace | Closing not complete by 10:30 pm. 7 of 12 done. Tap to see what's left. | At deadline + grace |
| Not started by deadline + grace | Closing not started by 10:30 pm. | At deadline + grace |
| Report arrives after a not-complete alert (phone was offline or relay was down) | Update: closing was completed at 10:12 pm. The report arrived late. | When the report reaches the relay |
| Relay was down over the deadline and no report came after it restarted | No closing report for last night. Our service was offline 9:40 pm to 6:55 am. Open the app to check. | 30 min after the relay restarts |
| Staff phone asks to join | Maria's phone wants to join Main St. Open to approve. | Immediately |
| Test alert from setup | Test alert: you're set up. | On tap |
| Owner marked today closed | Nothing | Never |

Staff get one push of their own: "Closing due at 10:30 pm. 5 items left." 30 minutes before the deadline if the closing isn't finished (owner can turn it off).

- **Status only on the lock screen.** Push text carries status, counts, times and the shop label. Names, item text, notes and photos stay encrypted and show only inside the app, so Apple and Google never see them.
- **Not-complete alerts are Time Sensitive on iPhone**, so they break through Focus modes. The owner can turn this off.
- **Default grace is 15 minutes** after the deadline (open question 6).

## Offline and relay-down behavior

The relay being off never blocks a closing and never loses one; it only delays the owner's message.

- **Local first.** Every action writes to the phone's own database instantly, then adds a row to an outbox. The staff member sees the tick land whether or not anything is connected.
- **Outbox.** Each message has a unique ID, a per-device sequence number and the time it was made. While the app is open it retries right away, then after 5 s, 15 s, 1 min and every 5 min. In the background it uses Android WorkManager (runs when a network appears) and iOS background refresh, plus a retry every time the app opens or a push wakes it.
- **iOS limit, stated honestly.** iOS decides when an app gets background time. A report queued on an iPhone whose app was closed may not send until the app is opened again. So the Finish screen always shows either **Sent** or **Saved on this phone, not sent yet. Keep the app open a minute or open it again later**, and the phone schedules its own reminder if the queue is still waiting the next morning.
- **Safe to resend.** The relay and every phone drop duplicates by message ID, so retrying is always safe.
- **True times.** Every action carries the phone's time plus the phone's clock offset from the relay, measured at its last sync. A report delivered hours late still shows the real finish time. If the phone's clock was more than 5 minutes off, the report says so.
- **Lock-in.** Finish closing freezes the closing and the staff phone signs it. When the owner's phone receives it, it checks the signature and stores it as final, with its own received-at time. After that, only an appended correction note is possible, never an edit. The owner's phone holds the record of truth.
- **Relay restart.** When the relay comes back, it holds any not-complete alerts for deadlines that passed while it was down for 30 minutes, so queued reports can arrive. Then it sends the right message from the table above: finished (late delivery) or no report.
- **Visible state.** The owner's app shows **Connected** or **Can't reach the service since 9:42 pm. Reports will arrive when it's back.**
- **Austin knows first.** The relay pings a free uptime monitor (or Austin's existing alert channel) every minute; Austin is alerted if it's silent for 5 minutes (open question 16).

## The relay

One small Rust service on a dedicated Raspberry Pi carries sealed messages between phones, watches closing deadlines and sends pushes. It can read routing data and closing status, never content.

### What it can read

| Can read | Never sees in readable form |
| --- | --- |
| Random shop and device IDs, device platform, push tokens | Staff names |
| The shop label the owner chose | Checklist item text |
| Time zone, open days, deadlines, grace, closed dates | Notes, numbers entered, issue descriptions |
| Closing status: started, X of Y done, finish time, issue count, late or on time | Photos |
| Gift code redemptions and the salesperson tag on each code | Owner's name, phone or email (never collected) |
| Message sizes and times | The shop's encryption key |

### Storage and retention

- SQLite in WAL mode for the shop registry, schedules, status and queue; sealed message bodies as files on disk.
- A message is deleted as soon as every recipient device confirms it. Undelivered messages are dropped after 14 days.
- Closing status rows are kept 45 days (needed for late-report updates), then deleted.
- **Delete shop** from the owner's app wipes everything the relay holds for that shop immediately.

### API

HTTPS and JSON, versioned under `/v1`. Claude Code writes the full wire format in `docs/PROTOCOL.md` before any relay code.

| Endpoint | Caller | Does |
| --- | --- | --- |
| `POST /v1/redeem` | New owner device | Gift code in; creates the shop, registers the admin device, returns its device token |
| `POST /v1/join` | New staff device | Join token and public keys in; creates a pending join request |
| `POST /v1/join/{id}/approve` | Admin | Approves the join and attaches the sealed shop key |
| `POST /v1/envelopes` | Any device | Sends sealed messages to shop members, in batches |
| `GET /v1/inbox` | Any device | Fetches messages waiting for this device (long-poll up to 25 s) |
| `POST /v1/ack` | Any device | Confirms receipt; the relay deletes once everyone has confirmed |
| `POST /v1/status` | Staff device | Closing status: started, items done, finished, issue count |
| `PUT /v1/schedule` | Admin | Time zone, open days, deadlines, grace, closed dates |
| `PUT /v1/push-token` | Any device | Updates its push token |
| `DELETE /v1/devices/{id}` | Admin | Removes a device; its token stops working immediately |
| `DELETE /v1/shop` | Admin | Deletes the shop and everything the relay holds for it |
| `GET /v1/health` | Anyone | Liveness check |

- **Auth:** each device gets a random 256-bit token at redeem or join, kept in the iOS Keychain or Android Keystore and sent over TLS only.
- **Abuse limits:** per-device and per-IP rate limits; 5 failed redeem or join attempts per IP per hour; message size capped at 2 MB.

### Deadline watcher

- Runs every 30 s. Each shop's deadline is computed in its own IANA time zone, so daylight-saving nights are handled.
- At deadline + grace with no finished status, it sends the not-complete push. A finish after the deadline sends the finished-late push.
- A closing finished after midnight belongs to the business day it started. The cutoff is deadline + 6 hours.
- At deadline − 30 min with no finished status, it sends the staff reminder.

### Push

- Firebase Cloud Messaging for Android and iPhone, with Austin's APNs key uploaded to Firebase, so the relay has one push path. FCM is free.
- The Firebase service-account key lives on the relay host, readable only by the relay's user.

### Hosting

- **Default host:** Raspberry Pi 5 with 4 GB (a Pi 4 with 2 GB or more also works), booting from a USB SSD rather than an SD card, on wired ethernet, with the official power supply. A small UPS is optional.
- Raspberry Pi OS Lite 64-bit. The relay runs as a systemd service under an unprivileged user and restarts on failure.
- **Reachable from 5G through Cloudflare Tunnel:** free, connects outward, so no router port is opened and the home IP stays hidden. Address: `relay.<company domain>`. Alternative: port-forward 443 with Caddy for certificates and dynamic DNS.
- **Apps know the relay only by domain name, never by IP.** Moving it to the second workstation or anywhere else later is a DNS change, not an app update.
- **Isolated:** not on The-Money-Engine and not on the vpn-gateway Pi. Ideally on a separate guest network or VLAN.
- **Backup:** nightly encrypted copy of the relay database to Austin's workstation. Losing it would cut every shop off.
- **Capacity:** real load is a few hundred small messages a day. Target: 500 shops on one Pi with room to spare.
- **Operator tool `relayctl`** on the relay host: create gift codes in batches tagged by salesperson, revoke codes, list shops (health only by default, open question 11), show queue depth, run a backup, rotate push credentials.

## Security and encryption

Content is end-to-end encrypted between phones with libsodium, so a breach of the Pi exposes no checklist content, names or photos.

- **Device keys.** Each phone makes an X25519 key pair (encryption) and an Ed25519 key pair (signing). Private keys live in the iOS Keychain or Android Keystore-protected storage and never leave the phone.
- **Shop key.** The first admin device creates a 256-bit shop key. All content is encrypted with XChaCha20-Poly1305 under it.
- **Joining.** The join QR holds the relay's domain, the shop ID, a one-time join token and a pairing secret. The staff phone sends its public keys with a proof made from the pairing secret. The owner's phone checks the proof, the owner taps Approve, and the owner's phone seals the shop key to the staff phone's public key. The relay never sees the shop key.
- **Removing a device.** The relay revokes its token at once. The admin phone makes a new shop key and seals it to each remaining device. The removed phone keeps what it already had and sees nothing new.
- **Signed reports.** Each finished closing is signed by the staff phone that finished it, and the owner's phone checks the signature before accepting it. The relay can delay a report but can't forge or edit one. Gaps in a device's sequence numbers flag missing messages.
- **Photos.** In-app camera only, capture time stamped, resized to 1600 px on the long edge, JPEG quality about 85, encrypted before leaving the phone.
- **No home-made crypto.** libsodium primitives only.
- **Lost owner phone.** Setup recommends a second admin device. If the only admin phone is lost, the owner restarts with a new gift code; history on that phone is gone unless exported.

### Threats

| Threat | Result |
| --- | --- |
| Relay Pi hacked or stolen | Attacker sees shop IDs, schedules, closing times and status, push tokens. No content. Rotate push credentials and restore from backup. |
| Relay off | Closings continue on the phones. Messages are delayed, not lost. |
| Gift code leaks | Codes are single-use; unused ones can be revoked with `relayctl`. |
| Staff phone lost, or staff member leaves | Owner removes the device; the shop key rotates. |
| Staff changes the phone clock to fake a closing time | The clock offset shows on the report. |
| Staff reuses an old photo | Not possible; the gallery is never offered. |
| Code guessing or flooding | Rate limits, with Cloudflare in front. |

Design goal: the relay holds none of the personal data types that trigger breach-notice laws such as California Civil Code 1798.82. Confirm with a lawyer before launch; the privacy policy must state exactly what the relay can see.

## App screens and data

Staff mode has four screens and admin mode has ten; the **This month** screen is the one that carries the gift's value.

### Staff mode

| Screen | What it shows |
| --- | --- |
| Tonight | The checklist, progress (7 of 12), who ticked each item and when |
| Item | Camera for photo items, number entry for number items, Can't do this with a reason |
| Finish | Summary of done, can't-do and issues; submit |
| Recent | This phone's last 7 closings with Sent or Waiting status |

A send-status banner sits on every staff screen: **Sent** or **Saved, not sent yet.**

### Admin mode

| Screen | What it shows |
| --- | --- |
| Tonight | Live status: not started, in progress (X of Y), complete, late, issues; last activity |
| Report | One closing in full: each item, who, when, photos, issues, clock warnings |
| This month | On time X of Y open nights, late, missed, issues, the most common issue, current on-time streak, average finish time |
| Checklist | Editor plus starter templates; edits are versioned, and a closing uses the version active when it opened |
| Schedule | Days open, deadline per day, grace, checklist opening time, closed dates |
| Team | Devices, pending joins to approve, remove device, add admin |
| Alerts | Test alert, staff reminder on or off, Time Sensitive on or off |
| Export | A month's closing log as PDF and CSV through the share sheet |
| Shops | Switch between locations |
| Service | Relay connection state and last contact time |

**This month** example: "28 of 30 closings on time. 2 late, 0 missed. 3 issues; most common: deposit not made." It's what the owner sees value in, the check-in agenda, and the buyer-ready record once exported.

### Checklist item types

| Type | Done when | Issue when |
| --- | --- | --- |
| Check | Ticked | Marked can't-do |
| Photo | Photo taken in-app | Marked can't-do |
| Number with range (drawer count, fridge temperature) | Value entered | Value outside the range, or can't-do |
| Yes / No | Answered | Answered No, or can't-do |
| Note | Optional text | Marked as an issue by staff |

Starter templates, 8–15 items each: restaurant or café, retail store, salon or spa, contractor office or yard, and blank. Claude Code drafts them; Austin reviews (open question 14).

### On-device data (SQLite)

- `shops`: ID, label, role on this device, relay domain
- `devices`: the encrypted roster (device ID, first name, role, public keys)
- `checklist_versions` and `items`
- `closings`: shop, business date, state, finished time, finishing device, signature
- `item_events`: closing, item, action, value, device, time, clock offset, photo reference
- `photos`: encrypted files on disk
- `outbox`, `inbox_seen`, `settings`

**Retention:** reports kept forever on admin phones (small); photos 90 days by default, adjustable; staff phones keep 7 days.

## Tech stack and building from Linux

Default: Flutter for the app and Rust for the relay. Android builds on Austin's Linux machine; iOS builds and uploads run on a GitHub Actions macOS runner, because Apple's build tools only run on a Mac.

| Part | Default | Alternative |
| --- | --- | --- |
| App | Flutter (Dart), one codebase for iPhone and Android | React Native with Expo, whose cloud build service can build iOS without a Mac |
| On-device database | SQLite through drift | — |
| Encryption | libsodium through `sodium_libs` | — |
| Camera | Flutter `camera` plugin, in-app capture only | — |
| QR scanning | `mobile_scanner` | — |
| Push | `firebase_messaging` (FCM on both platforms) | Direct APNs for iPhone |
| Background sending | `workmanager` (Android WorkManager, iOS background tasks) | — |
| Relay | Rust: axum, tokio, rusqlite, chrono-tz; one static binary for the Pi (aarch64) and the workstation (x86\_64) | Go |
| iOS build and upload | GitHub Actions macOS runner with fastlane and an App Store Connect API key | Expo's build service |
| Android build and upload | Built on Linux; fastlane uploads with a Play service account | — |
| Landing page and privacy policy | Static `site/` on the company website, same as the other tools | — |

- **Minimum OS:** iOS 16, Android 8.0 (API 26). Closing staff often carry older Androids.
- **Target API 36:** Google requires new apps and updates to target Android 16 (API 36) from Aug 31, 2026 ([Android developers](https://developer.android.com/google/play/requirements/target-sdk)).
- **Testing from Linux:** Android emulator plus at least one real low-end Android phone. iPhone testing runs in the simulator on CI and through TestFlight on Austin's iPhone.
- **Reuse first:** if the voice-capture iPhone app already has a working iOS build pipeline from Linux, reuse it (open question 1).

## App Store and Google Play

The app ships free on both stores with no prices or paid unlock inside it; the gift code sets up a shop, like a business sign-in, rather than unlocking a purchase.

### Both stores

- **No price or "unlock" language anywhere in the app or its listings.** No "a $X tool, free for you." That framing lives on the card and the website only.
- Privacy policy URL on the company site, stating exactly what the relay can see.
- In-app deletion: **Delete shop** for owners and **Leave shop** for staff, each wiping what the relay holds.
- Reviewers need a working demo: put a demo gift code and join steps in the review notes, and keep the relay up during review.

### Apple (existing account)

- App Store Connect record, bundle ID, App Privacy answers, camera and notification permission text, Push Notifications and Time Sensitive Notifications capabilities, background modes.
- Apple's guideline 3.1.1 says apps may not use their own mechanisms, such as QR codes or license keys, to unlock paid content or functionality. Keeping the app free with the code as a setup step avoids it. This is from memory: Claude Code must read the current guidelines (3.1.1, 3.1.3, 5.1.1) before the first submission.

### Google Play ($25, one time)

- **Closed-test rule:** personal accounts created after Nov 13, 2023 must run a closed test with at least 12 opted-in testers for 14 continuous days before getting production access. Organization accounts are exempt ([ExtendsClass, 2026](https://extendsclass.com/blog/google-plays-closed-testing-requirement-what-developers-need-to-know-in-2026)). If Austin's account is personal, start the 14 days in Phase 0, not at the end (open question 3).
- Target API 36 from Aug 31, 2026 (see the tech stack section).
- Data safety form, content rating questionnaire, privacy policy.

### Seller name

Individual accounts show Austin's legal name on the listing; organization accounts show the company name and usually need a D-U-N-S number (open question 3).

## Hard constraints and acceptance tests

Eleven constraints hold for every build, and the app ships only when every acceptance test passes on real phones.

### Hard constraints

1. No recurring cost: Apple (already paid), Google Play ($25 once), FCM (free), Cloudflare Tunnel (free), the existing company domain.
2. The relay never stores readable content. Messages are deleted on delivery, 14 days at most.
3. A whole closing can be done with no connection, and nothing is lost while the relay is off.
4. Owner alert within 60 s of Finish when the relay is up and the phones are online.
5. Not-complete alert within 2 minutes of deadline + grace when the relay is up.
6. Owner setup under 5 minutes including the test alert; staff join under 60 s.
7. One relay serves all shops: 500 shops on a Pi 4 with 2 GB, under 50% CPU in a simulated peak closing hour.
8. The same relay binary runs on the Pi (aarch64) and the workstation (x86\_64); apps reach it by domain name only.
9. No staff accounts, emails or phone numbers collected.
10. Photos captured in-app only.
11. Relay isolated from The-Money-Engine and the vpn-gateway Pi.

### Acceptance tests

| Test | Passes when |
| --- | --- |
| Normal close, iPhone staff to Android owner, and the reverse | Owner push within 60 s; report matches; photos open |
| Two staff on one checklist at once | Ticks merge, nothing lost, one report |
| Whole closing in airplane mode, reconnected 3 h later | Report arrives with the true finish time; owner gets the Update message after the earlier not-complete alert |
| Relay stopped 9 pm to 7 am across the deadline | Nothing lost; 30 min after restart the correct late-delivery or no-report message; no duplicates |
| Power pulled on the Pi mid-upload | Database intact; duplicates dropped; resend safe |
| Not started, and partly done, at deadline + grace | Correct not-complete text with counts |
| Daylight-saving nights; two shops in different time zones | Alerts at the correct local times |
| Staff phone clock set 30 min back | Report flags the clock offset |
| Device removed | Its next request is refused at once; it can't read anything sent after the key rotation |
| Gift code reused, and 1,000 guesses from one IP | Single use enforced; guesses rate-limited |
| Push with the app force-closed, both platforms | Push arrives |
| Load: 500 simulated shops, 5 phones each, closing in the same hour, on a Pi 4 | Constraint 7 met |
| Search the relay database and disk after a test run for known item text, names and notes | No plaintext found |
| Store pre-flight | TestFlight build accepted; Play pre-launch report clean |

## Build phases

Nine phases, each ending in a gate Claude Code must pass and report to Austin before starting the next. Phase 0 proves the riskiest part, push from Austin's own hardware to both phone types over 5G, before any product code.

0. **Prove the pipeline.** Ask Austin open questions 1–4. Hello-world app on TestFlight and on Play internal testing. Relay stub on the Pi behind Cloudflare Tunnel. If the Google account is personal, start the 12-tester closed test now.
   - Gate: a push sent from the Pi reaches Austin's iPhone and an Android phone on 5G with the app force-closed.
1. **Relay core.** `docs/PROTOCOL.md` first, then registry, redeem, join, envelopes, inbox, ack, duplicate handling, auth, rate limits, `relayctl`.
   - Gate: integration tests pass; a search of the database finds no plaintext.
2. **App core.** Local database, outbox and inbox sync, device keys, pairing with owner approval, roles, device removal and key rotation.
   - Gate: two phones paired; messages survive repeated relay restarts.
3. **Staff closing.** Checklist, all item types, in-app photos, Finish, signing, send-status banner.
   - Gate: the airplane-mode closing test passes.
4. **Owner side and alerts.** Tonight, Report, the deadline watcher and every message in the messages table, plus the staff reminder.
   - Gate: each message in the table is triggered by an automated test and seen on a real phone.
5. **Value and setup.** This month, PDF and CSV export, starter templates, schedule editor, the guided 5-minute setup with test alert.
   - Gate: Austin approves the templates and the This month screen.
6. **Failure drills.** The full acceptance table on real phones, including the load test on the Pi.
   - Gate: every test passes.
7. **Store submission.** Listings, screenshots, privacy policy, App Privacy, Data safety, reviewer notes with a demo code.
   - Gate: live on both stores.
8. **Gift kit.** Card landing page at `/c/<code>` that routes to the right store, gift-code batches per salesperson in `relayctl`, a salesperson demo shop on two devices, a one-page recovery card for setup problems.
   - Gate: a salesperson runs the demo and a fresh owner setup in under 5 minutes.

## Open questions for Austin

Claude Code asks each question when it reaches the phase it blocks, offering the default; if Austin says "your call," the default stands and goes in `DECISIONS.md`.

| # | Question | Default | Blocks |
| --- | --- | --- | --- |
| 1 | Did the voice-capture iPhone app already get iOS builds working from Linux? Reuse that pipeline? | Flutter, GitHub Actions macOS runner, fastlane | Phase 0 |
| 2 | Relay host: a new dedicated Pi or the second workstation? Cloudflare Tunnel or port-forwarding? Which company domain? | New Pi 5 4 GB with SSD; Cloudflare Tunnel; `relay.<company domain>` | Phase 0 |
| 3 | Apple account: individual or organization? Google Play: personal (12 testers for 14 days) or organization (exempt, needs D-U-N-S)? | Organization on Google if D-U-N-S is quick; otherwise start the closed test in Phase 0 | Phase 0 |
| 4 | App name, icon and bundle ID. The bundle ID can never change after the first upload. | Working name Closing Tracker | Phase 0 |
| 5 | One shop per gift code, or several locations? Tag codes by salesperson? | Up to 3 locations per code; tagged by salesperson | Phase 1 |
| 6 | Grace after the deadline; checklist opening time; staff reminder | 15 min grace; opens 3 h before; reminder on | Phase 4 |
| 7 | What counts as an issue? | Can't-do items, out-of-range numbers, No answers, notes marked as issues | Phase 3 |
| 8 | Lock screen: status only, or names and issue text too (needs encrypted push and extra iOS work)? | Status only | Phase 4 |
| 9 | Roles: admin and staff only, or a manager role (gets alerts, can't edit the checklist)? | Admin and staff; a manager is a second admin | Phase 2 |
| 10 | Retention on the owner's phone | Photos 90 days, reports forever, staff phones 7 days | Phase 5 |
| 11 | Operator view: health only, or per-shop on-time stats Austin can see before check-ins (must be disclosed)? | Health only | Phase 1 |
| 12 | Location check at Finish to prove staff were at the shop (needs location permission)? | Off | Phase 3 |
| 13 | Spanish staff screens in v1? | English v1, every string externalized for Spanish | Phase 3 |
| 14 | Which trades get starter templates, and who reviews the items? | Restaurant, retail, salon or spa, contractor office or yard; Austin reviews | Phase 5 |
| 15 | If the app is ever sold, through Apple and Google in-app purchase or another route? Check the rules current at that time. | Free only in v1 | Phase 7 |
| 16 | Where relay-down alerts go | A free uptime monitor emailing Austin | Phase 1 |

## Working rules for Claude Code

Ask instead of assuming, keep v1 to this spec, and leave a written trail of every decision.

- **Ask, don't assume.** Ask the open questions when you reach the phase they block, one batch at a time, each with its default. Anything this spec doesn't cover and isn't obvious: ask Austin.
- **DECISIONS.md.** Log every decision with its reason and date, including each default Austin accepts.
- **Stay in scope.** No features beyond this spec without asking.
- **Hands off.** Never touch The-Money-Engine's trading work or the vpn-gateway Pi. The relay deploys only to its dedicated host.
- **Repo layout:** `app/` (Flutter), `relay/` (Rust), `site/` (landing page, privacy policy, support), `ops/` (Pi setup script, systemd unit, cloudflared config, backup), `docs/` (PROTOCOL.md, THREATS.md, DECISIONS.md, STORE.md), `.github/workflows/`.
- **Secrets never committed.** App Store Connect API key, Play service account, Firebase key and signing keys go in GitHub Actions secrets, or in files on the relay host readable only by the relay's user.
- **Self-tests.** `relay selftest` runs a full round trip. The app has a hidden diagnostics screen (tap the version number 7 times) that runs the same check from the phone.
- **Gates are real.** Don't claim a gate passed without the tests that prove it.
- **Phase report.** End each phase with a short note to Austin: what passed, what's open, what you need from him.
