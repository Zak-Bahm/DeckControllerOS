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
./scripts/dev_http_serve.sh --dir out/dev-payload --port 8000 --bind 0.0.0.0
```

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

## When You Still Need Rebuild + Reboot

Perform `./scripts/build.sh` and reboot only for changes to:

- `configs/kernel/*`
- `configs/buildroot/*`
- `br2-external/*`
- init scripts under `configs/init/*`
- any files copied into the image by `post-build.sh`
