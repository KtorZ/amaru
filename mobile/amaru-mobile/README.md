# Amaru Mobile

An iOS Tauri dashboard for a nearby Amaru node. It connects to
`amaru-mobile-telemetry` over Bluetooth Low Energy and renders the same
operator-facing state as the terminal **Amaru** view, excluding logs.

The phone receives one bounded CBOR state snapshot per second. It does not
receive JSON traces, raw metrics, ledger data, or general control access to the
node. It can optionally request a host power-off through one fixed BLE command.

## Prerequisites

- Xcode with an Apple development team configured.
- CocoaPods (`brew install cocoapods`) for Tauri's generated iOS project.
- The [Amaru mobile telemetry bridge](../../tools/amaru-mobile-telemetry/) on
  the Linux host, with BlueZ and Bluetooth enabled.
- A physical iPhone or iPad: the iOS simulator cannot use Bluetooth LE.

## Setup

Install JavaScript dependencies and initialise Tauri's generated iOS project:

```bash
cd mobile/amaru-mobile
npm install
npm run tauri ios init
```

The Bluetooth usage descriptions are maintained in
[`src-tauri/Info.plist`](src-tauri/Info.plist) for macOS and
[`src-tauri/Info.ios.plist`](src-tauri/Info.ios.plist) for iOS. Tauri merges
them into generated plists, so do not edit files below `src-tauri/gen/`.

`CoreBluetooth.framework` is declared in
[`src-tauri/ios-project.yml`](src-tauri/ios-project.yml), the tracked XcodeGen
template. Regenerate the iOS project with `npm run tauri ios init -- --ci` after
changing that template.

For a physical device, connect and unlock the iPhone, trust the Mac, enable
**Developer Mode** in **Settings → Privacy & Security**, then run:

```bash
npm run tauri ios dev "Your iPhone Name"
```

The Vite configuration already honours Tauri's `TAURI_DEV_HOST` so the phone
can load the development bundle from the Mac over the local network. Xcode can
also run the generated project directly after selecting the connected iPhone as
its destination.

The bridge's service UUID and stream UUID are defined once in
[`src/protocol.ts`](src/protocol.ts). The CBOR field order is also defined
there and must remain compatible with
[`tools/amaru-mobile-telemetry/src/wire.rs`](../../tools/amaru-mobile-telemetry/src/wire.rs).
The application and bridge currently support only snapshot version `5`, so
deploy them together when upgrading either side of the protocol.

`npm test` first asks the Rust bridge test suite to write its CBOR vectors
under `tools/amaru-mobile-telemetry/target/test-vectors/`, then validates that
the TypeScript decoder accepts those exact artifacts.

## Security

The telemetry characteristic is not encrypted by the BlueZ local-GATT API used
in this first version. Treat it as trusted, local-network telemetry. Do not
enable it where peer addresses or node health information must be confidential.

The dashboard also has an optional, explicitly-confirmed **Power off** action.
It writes one fixed public command to the bridge and cannot pass arguments or
run arbitrary code. It is intentionally unauthenticated: a nearby Bluetooth
attacker can therefore power off the host if the bridge's optional power-off
systemd and sudo configuration has been installed and its power-off capability
has been enabled. See the bridge README for the installation and operational
details.
