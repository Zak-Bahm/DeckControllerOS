# BLE Pairing Test Harness

End-to-end, repeatable test for debugging BLE pairing failures — in particular
the failure where a host sees the controller, tries to connect, the connection
fails, and the controller then **stops advertising (disappears)**.

It captures both ends of one pairing attempt:

- **Deck (peripheral):** `scripts/tests/test_bt_pair_capture.sh`
- **Linux host (central):** `scripts/bt_pair_test.sh`

Unlike `test_hidd_reconnect.sh` (which assumes pairing already succeeded), this
harness drives a **fresh** pair from a clean slate and records the wire-level
HCI/SMP exchange, so you can see exactly *why* a pair aborts.

## What gets captured

Deck side (uploaded inline in the `controlleros-dev-run` log under `out/dev-logs/`):

- **btmon** HCI/SMP trace (text) — the advertise → connect → SMP → encryption → ATT exchange.
- **Verbose `bluetoothd -d`** log — SMP / authentication / "Pairing failed" reasons.
- **Full `hidd.log`** — whether `ServicesResolved` fired and whether the
  advertisement was re-registered after a disconnect
  (see the skip path in `crates/hidd/src/hog.rs`).
- **1 Hz transition log** — adapter `Discoverable` and per-device
  `Paired/Trusted/Connected/ServicesResolved`, so the exact moment the controller
  stops being visible is timestamped.
- `dmesg` bluetooth/hci/smp lines.

Host side (written to `out/dev-logs/`):

- `host_btmon_<ts>.txt` — the central adapter's HCI/SMP trace.
- `host_bt_pair_<ts>.log` — verdict, central state transitions, and the raw
  `bluetoothctl pair`/`connect` output.

## Prerequisites

- A **Linux host with BlueZ** (`bluetoothctl`, `btmon`) acting as the central.
- The dev HTTP loop set up (see [`dev_testing_loops.md`](dev_testing_loops.md)).
- The deck-side script is staged automatically — `dev_stage_payload.sh` copies
  all of `scripts/tests/*.sh` into the served payload.

## Running it

1. **Host — build, stage, serve:**
   ```sh
   cargo build --release -p hidd -p controllerosctl
   ./scripts/dev_stage_payload.sh --hidd target/release/hidd \
       --controllerosctl target/release/controllerosctl --hid-config configs/hid/hid.toml
   ./scripts/dev_http_serve.sh --dir out/dev-payload --logs-dir out/dev-logs --port 8000
   ```

2. **Deck — pull binaries, then start the capture (opens the pairing window):**
   ```sh
   controlleros-dev-update --base-url http://<HOST_IP>:8000
   controlleros-dev-run --base-url http://<HOST_IP>:8000 \
       --timeout-seconds 120 --shell-script test_bt_pair_capture.sh
   ```
   The deck POSTs a `READY: pair from host now (deck=<MAC>, window=…)` line to
   the server terminal when the window opens. Default window is 60 s; override
   with `WINDOW_SECONDS` (note `--timeout-seconds` must exceed the window).

3. **Host — within the window, drive the central:**
   ```sh
   sudo ./scripts/bt_pair_test.sh   # auto-discovers "ControllerOS Xbox Controller"
   ```
   `btmon` needs root. Useful flags: `--deck-mac <MAC>` (skip discovery),
   `--hci <dev>`, `--window <s>`, `--deck-name <name>`.

4. **Inspect** `out/dev-logs/`:
   - the deck capture log (transition log + hidd.log + verbose bluetoothd.log + deck btmon.txt)
   - `host_bt_pair_*.log` / `host_btmon_*.txt`

5. **Repeat** steps 2–3 freely. Each run self-cleans (removes bonds, truncates
   logs, restores services on the Deck), so iterations are independent.

## Reading the captures

Work from the symptom backwards:

- **Did a device connect at all?** The deck transition log shows
  `connected devices: …`. No connect → an advertising/scan problem, not pairing.
- **Did `ServicesResolved` ever reach `yes`?** If a device connects but
  `resolved` stays `no`, the GATT/encryption handshake failed mid-pairing — look
  in the **btmon** trace for `SMP: Pairing Failed` (with a reason code) or an
  `LE Connection Complete` quickly followed by `Disconnect Complete` (reason code,
  e.g. `0x05` authentication failure, `0x3d` MIC/connection-failed-to-establish).
  The verbose **bluetoothd.log** usually names the reason in plain text.
- **Did the controller stop advertising?** The transition log timestamps
  `discoverable: yes -> no`. Correlate with `hidd.log`: if you see
  `device disconnected … skipping re-register, no stable connection`, hidd left
  the adapter non-discoverable after a pre-`ServicesResolved` failure
  (`crates/hidd/src/hog.rs`).

Cross-reference the deck and host btmon traces by their timestamps to see which
side aborted the SMP exchange.
