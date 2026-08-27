# Amaru mobile telemetry bridge

`amaru-mobile-telemetry` follows Amaru's JSON traces in the system journal,
reduces them to the state required by the **Amaru** dashboard, and publishes a
versioned CBOR snapshot through a Bluetooth Low Energy GATT notification
characteristic.

It produces one fresh snapshot per second, caps the encoded snapshot at 7 KiB,
and fragments it into 160-byte payloads for conservative BLE MTUs. Fragments
are spaced by 20 ms so receivers can drain them; the complete wire rate,
including fragment headers, stays below 10 KiB/s.

The stream accepts one notification subscriber at a time, so the service-wide
BLE output remains within that budget as well.

The service consumes only Amaru's public telemetry schema names and generated
field constants. Unknown, malformed, or unrelated JSON lines are ignored.

## Trace source

Run Amaru with JSON traces. The bridge reads `amaru.service` through
`journalctl`, so no trace file or log-rotation configuration is required. It
samples process/host resource data itself because those metrics are not part of
the JSON trace stream.

The bridge rebuilds its in-memory dashboard projection from the most recent
4,096 journal entries on every start, preserving each journal timestamp. It
then checkpoints the final cursor under its systemd state directory and follows
new entries from there. This restores the current tip, peers, and mempool, and
reconstructs recent throughput and rollback statistics without treating the
replay itself as live work.

```ini
# /etc/systemd/system/amaru.service.d/mobile-telemetry.conf
[Service]
Environment=AMARU_WITH_JSON_TRACES=true
Environment=AMARU_TRACE=warn,amaru=debug,amaru_pure_stage=warn
StandardOutput=journal
```

The `debug` trace filter is necessary for `tip.update`, which provides the
debounced block and transaction deltas used for throughput. This is an opt-in
operator setup: JSON traces are materially more verbose than normal node
logging.

The bridge unit grants its `amaru` process journal access through the
`systemd-journal` group. Override `AMARU_MOBILE_JOURNAL_UNIT` when the node
runs under another unit name. `AMARU_MOBILE_JOURNAL_CURSOR_FILE` defaults to
the bridge's systemd-managed state directory and can be overridden for a
manual invocation.

## Running

The bridge is its own Cargo workspace so it does not enter Amaru's regular
build or release dependency graph.

```bash
cargo build --manifest-path tools/amaru-mobile-telemetry/Cargo.toml --release
sudo install -m 0755 tools/amaru-mobile-telemetry/target/release/amaru-mobile-telemetry /usr/local/bin/
sudo install -m 0644 tools/amaru-mobile-telemetry/systemd/amaru-mobile-telemetry.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now amaru-mobile-telemetry.service
```

Set `AMARU_PID` when automatic discovery cannot find the node process.

### Optional power-off action

The mobile application can request a host power-off through a distinct GATT
characteristic. It is disabled by default. When enabled, it accepts one fixed
command only: it cannot pass arguments, run arbitrary programs, or operate the
node. The action is deliberately unauthenticated, so any nearby Bluetooth
client that knows the public command can power off the host. Install it only
where that availability risk is acceptable:

```bash
sudo install -m 0644 tools/amaru-mobile-telemetry/systemd/amaru-mobile-poweroff.service /etc/systemd/system/
sudo install -m 0440 tools/amaru-mobile-telemetry/sudoers.d/amaru-mobile-poweroff /etc/sudoers.d/
sudo visudo --check --file=/etc/sudoers.d/amaru-mobile-poweroff
sudo systemctl daemon-reload
sudo systemctl edit amaru-mobile-telemetry.service
```

Add the following drop-in, then restart the bridge:

```ini
[Service]
Environment=AMARU_MOBILE_ENABLE_POWER_OFF=true
```

```bash
sudo systemctl restart amaru-mobile-telemetry.service
```

The bridge runs as `amaru`. Its sudo rule permits exactly
`systemctl --no-block start amaru-mobile-poweroff.service`; the root-owned
one-shot unit can only request a system power-off. Neither component grants a
shell nor a general systemd control capability.

The power-off unit is triggered on demand and must not be enabled. The bridge
unit intentionally sets `NoNewPrivileges=false` and `RestrictSUIDSGID=false`:
its restricted sudo handoff needs to execute as root.


### From MacOS

This crate requires a few linux utilities

```console
docker run --rm \
  -v "$PWD":/workspace \
  -w /workspace \
  amaru-dev-linux:latest \
  bash -c 'apt-get update && apt-get install -y --no-install-recommends libdbus-1-dev pkg-config && cargo check --manifest-path tools/amaru-mobile-telemetry/Cargo.toml'
```

## Bluetooth contract

| Item | Value |
| --- | --- |
| Service UUID | `8b4cb36a-7a5d-4f9f-8f31-6a5f4fc8c711` |
| Stream UUID | `8b4cb36a-7a5d-4f9f-8f31-6a5f4fc8c712` |
| Power-off UUID | `8b4cb36a-7a5d-4f9f-8f31-6a5f4fc8c713` when `AMARU_MOBILE_ENABLE_POWER_OFF=true` |
| Stream characteristic | Notify |
| Power-off characteristic | Write, exact UTF-8 `amaru/power-off/v1` |
| Snapshot | CBOR array, version `3` |
| Fragment | `0xa7`, version, big-endian sequence, index, count, payload |

BlueZ's local GATT notification API does not expose an encryption requirement
for notification characteristics, so the telemetry stream must be treated as
local, trusted-network data. It should not be enabled where disclosure of peer
addresses or node health is unacceptable. The optional power-off action is also
unauthenticated and exposes a deliberate nearby-device denial-of-service risk.
