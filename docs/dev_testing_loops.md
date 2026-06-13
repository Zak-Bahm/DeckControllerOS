# Development Testing Loops (No Rebuild/Reboot by Default)

This guide reduces iteration time by separating development into two loops:

- Loop 1: host-only checks
- Loop 2: live Deck updates over local HTTP

Use full ISO rebuild + reboot only when kernel/Buildroot/init/rootfs wiring changes.

## Prerequisites

- Deck already booted into ControllerOS
- Deck and dev machine on the same local network
- Persistent storage mounted (`/var/lib/controlleros`)
- `python3` on dev machine (for HTTP hosting)

## Loop 1: Host-Only (Fast)

Run Rust quality gates and unit tests on the dev machine first:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Goal: catch compile/test/lint issues before touching the Deck.

## Loop 2: Live Deck Update via HTTP

### 1) Build binaries on dev machine

Example (adjust target/profile to your setup):

```sh
cargo build --release -p hidd -p controllerosctl
```

### 2) Stage payload for HTTP hosting

```sh
./scripts/dev_stage_payload.sh \
  --hidd target/release/hidd \
  --controllerosctl target/release/controllerosctl \
  --hid-config configs/hid/hid.toml
```

This creates:

- `out/dev-payload/manifest.txt`
- `out/dev-payload/bin/hidd`
- `out/dev-payload/bin/controllerosctl`
- optional `out/dev-payload/configs/hid/hid.toml`

### 3) Host payload over HTTP

```sh
./scripts/dev_http_serve.sh \
  --dir out/dev-payload \
  --logs-dir out/dev-logs \
  --port 8000 \
  --bind 0.0.0.0
```

This serves payload files and accepts Deck log uploads at:

- `POST http://<DEV_MACHINE_IP>:8000/logs`
- stored on host under `out/dev-logs/`

### 4) Pull and apply update on Deck

On ControllerOS shell:

```sh
controlleros-dev-update --base-url http://<DEV_MACHINE_IP>:8000
```

Optional with post-update restart command:

```sh
controlleros-dev-update \
  --base-url http://<DEV_MACHINE_IP>:8000 \
  --restart-cmd "/etc/init.d/S40bluetoothd restart"
```

Optional dry-run validation:

```sh
controlleros-dev-update --base-url http://<DEV_MACHINE_IP>:8000 --dry-run
```

### 5) Verify deployed files on Deck

```sh
controlleros-dev-list
```

## File Locations on Deck

By default, updates are installed under:

- `/var/lib/controlleros/dev/bin/*`
- `/var/lib/controlleros/dev/configs/hid/hid.toml`

No reboot is required for payload-only updates.

## Deck remote command log upload

`controlleros-dev-run` executes a shell command on the Deck and uploads full
combined stdout/stderr output to the host dev server.

Example:

```sh
controlleros-dev-run \
  --base-url http://<DEV_MACHINE_IP>:8000 \
  "bluetoothctl show && /etc/init.d/S45hidd restart && /var/lib/controlleros/dev/bin/controllerosctl hid self-test --hidd /var/lib/controlleros/dev/bin/hidd --config /var/lib/controlleros/dev/configs/hid/hid.toml"

controlleros-dev-run \
  --base-url http://<DEV_MACHINE_IP>:8000 \
  --timeout-seconds 10 \
  "/var/lib/controlleros/dev/bin/hidd"
```

Behavior:

- runs the shell command exactly once
- preserves the wrapped command exit code
- optionally enforces a timeout and returns exit code `124`
- uploads output to host `POST /logs`
- host stores a timestamped log file under `out/dev-logs/`

## Running staged test scripts

The reusable test scripts under `scripts/tests/` (e.g. `test_hidraw.sh`,
`test_hidd_reconnect.sh`, the `test_gui_step*` checkpoint scripts) are copied into
`out/dev-payload/` by `dev_stage_payload.sh` and served by `dev_http_serve.sh`
alongside the binaries. They can be downloaded and run on the Deck in one step:

```sh
controlleros-dev-run \
  --base-url http://<DEV_MACHINE_IP>:8000 \
  --shell-script test_hidraw.sh
```

Behavior:

- `--shell-script <name>` downloads `<name>` from `<base-url>/<name>`, makes it
  executable, and runs it on the Deck; any trailing positional args are passed
  through to the script.
- Script stdout/stderr is uploaded to the host `POST /logs` like any other
  `controlleros-dev-run` command, and stored under `out/dev-logs/`.
- Each script runs with `CONTROLLEROS_DEV_BASE_URL` exported, so it can POST
  live progress messages to the server's `/send-instruction` endpoint — these
  are printed on the host terminal running `dev_http_serve.sh` for real-time
  feedback while the script executes.

## When You Still Need Rebuild + Reboot

Perform `./scripts/build.sh` and reboot only for changes to:

- `configs/kernel/*`
- `configs/buildroot/*`
- `br2-external/*`
- init scripts under `configs/init/*`
- any files copied into the image by `post-build.sh`
