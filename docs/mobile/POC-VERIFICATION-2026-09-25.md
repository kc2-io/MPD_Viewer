# Mobile POC verification — 2026-09-25

## Source identity and scope

- Branch: `am/MPDViewer-mobile-poc`, rebased onto `origin/main` at `87458ac`.
- App identifier preserved: `com.modpackdad.mpdtabber.poc`.
- Existing desktop controller/player files, hosted source snapshot, Twitch client
  ID, preferences and release workflows were not modified.
- Media/chat in the POC are explicit fixtures. No Twitch request was made.

## Checks run

| Check | Result | Evidence boundary |
|---|---|---|
| `cargo test --locked -p mpd-viewer-mobile-poc` | 4 passed | Native-host Rust unit tests on Linux; selection uses `mpd-core`. |
| `cargo test --locked --workspace` | 87 passed | Full existing Rust workspace plus the mobile POC; loopback permission was needed by existing Twitch mock tests. |
| `cargo clippy --locked -p mpd-viewer-mobile-poc --all-targets --features custom-protocol` | Passed | One pre-existing `mpd-core` too-many-arguments warning. |
| JS syntax checks | Passed | Static syntax only. |
| `tests/mobile-poc.browser.cjs` | Passed at 390×844, 412×915 and 820×1180 | Headless Chromium responsive UI with browser fixture; no external requests. |
| `cargo tauri android init --ci --skip-targets-install` | Passed with Tauri CLI 2.11.5 | Generated Android Studio project. |
| `cargo tauri android build --debug --target x86_64 --apk --ci` | Passed | Actual Rust Android cross-compile and Gradle debug APK assembly. |
| `aapt dump badging` | Package ID/version/min/target verified | `com.modpackdad.mpdtabber.poc`, `0.1.0-alpha.7`, min 24, target 36. |
| `apksigner verify --verbose --print-certs` | Passed, APK Signature Scheme v2 | Android debug certificate only; not a production/release signature. |

Built artifact (ignored diagnostic build output):

```text
mobile-poc/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
SHA-256 a8e2da5a294f8d851cdbb1be3f3719d8d279a382d72d430de5496de260bdae6c
Size 131 MiB
```

The APK is large because it is an unoptimized universal debug artifact. It is
not a release asset and must not be published.

## Not run / blocked

- Android emulator runtime: `/dev/kvm` is absent on this host. The APK was not
  installed or launched, and no runtime behavior is claimed.
- iOS initialization/build/simulator: the current host is Linux with no Xcode;
  Tauri does not expose the `ios` subcommand here.
- Real Twitch playback, audio, chat, login, Turbo, viewer credit and mobile
  background behavior: intentionally outside the media-free POC.

## Toolchain used for the Android build

- Rust/Cargo 1.98.1
- Tauri CLI 2.11.5
- Tauri crate 2.11.5
- Temporary Temurin JDK 21.0.12.1
- Android SDK Platform 36, Build Tools 35/36, NDK 29.0.13846066
- x86_64 Android Rust target

The JDK/SDK/NDK were installed under `/tmp` for this validation and are not
repository content. The generated Android project is checked in; another machine
must install its own prerequisites.
