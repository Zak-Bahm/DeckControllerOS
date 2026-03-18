# Checkpoint 05 Plan — Basic On-Screen GUI (Slint)

## Development Workflow (applies to every step)
- Loop 1 (host-only, default):
  - Use `cargo check/test/clippy/fmt` for rapid iteration without Deck reboot.
  - Slint UI can be previewed on host using native window backend during development (default Slint features for host builds, LinuxKMS features for cross-compile).
- Loop 2 (live Deck update, default for runtime checks):
  - Build binaries locally, stage with `scripts/dev_stage_payload.sh`, serve with `scripts/dev_http_serve.sh`, apply on Deck with `controlleros-dev-update`.
  - Run `controlleros-gui` manually on Deck after update to validate.
  - Upload Deck-side logs to host via `controlleros-dev-run` for remote inspection.
- Full ISO rebuild + reboot (only when required):
  - Use `./scripts/build.sh` only for kernel config changes, init scripts, Buildroot package additions, or post-build integration.

### Test script convention
- **Every step that requires Deck validation must produce a test script** at `scripts/tests/test_gui_step<N>_<name>.sh`.
- Test scripts are placed in the dev payload directory and served from the host HTTP server.
- Test scripts are executed on the Deck via:
  ```sh
  controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step<N>_<name>.sh
  ```
- Output (stdout + stderr) is automatically uploaded to the host under `out/dev-logs/`.
- Scripts must use `PASS:` / `FAIL:` prefixed lines for each check so results are easy to grep from host logs.
- Scripts exit `0` on all-pass, non-zero on any failure.
- Checks that require **visual confirmation** or **physical touch interaction** must be printed as `MANUAL:` lines in the script output, describing exactly what to verify on the Deck's screen.

## Baseline assumptions
- Checkpoint 04 complete and validated. `hidd` runs with real input mapping and publishes via BLE GATT HOG.
- Init scripts: `S40bluetoothd`, `S41bluetooth-power`, `S45hidd` already in place.
- Kernel version: 6.6.79 (custom Buildroot build, defconfig at `configs/kernel/steamdeck_defconfig`).
- DRM (`CONFIG_DRM=y`, `CONFIG_DRM_I915=y`) and framebuffer (`CONFIG_FB=y`) already enabled. AMD GPU driver (`CONFIG_DRM_AMDGPU`) and touchscreen driver NOT yet enabled — must be added.
- Target: x86_64-unknown-linux-gnu (Buildroot glibc toolchain).
- Rust binaries cross-compiled via Buildroot's `cargo-package` infrastructure with offline vendoring.
- ControllerOS runs as root (MVP). This allows use of `backend-linuxkms-noseat`, avoiding the seatd dependency.

## Slint backend research findings

### Cargo features
- **Backend:** `backend-linuxkms-noseat` — DRM/KMS + libinput, no libseat required (safe because ControllerOS runs as root with direct `/dev/dri/*` and `/dev/input/*` access).
- **Renderer:** `renderer-femtovg` — GPU-accelerated 2D vector renderer using OpenGL ES via EGL. Chosen over `renderer-skia-opengl` because Skia's C++ build script causes host/target linker conflicts in Buildroot's cross-compilation environment (glibc linker scripts with absolute paths, pkg-config host/target confusion). FemtoVG is pure Rust, avoids all native build script issues, and is well-suited for the simple UI workload.
- Use `default-features = false` to avoid pulling in the default winit/femtovg/software stack.

### System library dependencies (resolved via pkg-config)
| Library | Purpose | Buildroot Package | Already Enabled? |
|---|---|---|---|
| `libinput` | Touch/input event handling | `libinput` (`BR2_PACKAGE_LIBINPUT`) | No — must add |
| `libudev` | Device enumeration for libinput | `eudev` | **Yes** (via `BR2_ROOTFS_DEVICE_CREATION_DYNAMIC_EUDEV=y`) |
| `libxkbcommon` | Keyboard layout handling | `libxkbcommon` (`BR2_PACKAGE_LIBXKBCOMMON`) | No — must add |
| `libdrm` | DRM/KMS modesetting | `libdrm` (`BR2_PACKAGE_LIBDRM`) | No — auto-selected by mesa3d |
| `libgbm` | Generic buffer management (OpenGL) | Provided by `mesa3d` | No — must add mesa3d |
| EGL + GLES | OpenGL ES rendering | Provided by `mesa3d` | No — must add mesa3d |

### Buildroot packages to enable
- `BR2_PACKAGE_MESA3D=y` with `BR2_PACKAGE_MESA3D_GALLIUM_DRIVER_RADEONSI=y` (auto-selects `libdrm`, `libdrm-amdgpu`, `llvm-amdgpu`)
- `BR2_PACKAGE_MESA3D_OPENGL_EGL=y` and `BR2_PACKAGE_MESA3D_OPENGL_ES=y` (EGL + GLES for FemtoVG)
- `BR2_PACKAGE_LIBINPUT=y`
- `BR2_PACKAGE_LIBXKBCOMMON=y`

### D-Bus crate for BlueZ
- `zbus` — pure-Rust async D-Bus client. No additional C library dependency beyond the D-Bus daemon (already running via `S30dbus` init script).

### Touch input
- Handled automatically by libinput through Slint's LinuxKMS backend. No special Slint API needed.
- If display rotation is needed (Steam Deck display is natively portrait 800×1280, typically rotated 270° to landscape 1280×800), use `SLINT_KMS_ROTATION=270` env var for display rotation. Touch coordinate rotation must be handled separately via udev rule setting `LIBINPUT_CALIBRATION_MATRIX`.

## Build system notes
- `crates/gui/` is a new binary crate that needs its own Buildroot package `.mk` file (`br2-external/package/controlleros-gui/`), following the same pattern as `controlleros-hidd.mk`.
- The existing vendoring hooks will vendor Slint, zbus, and all transitive dependencies automatically.
- New crate dependencies must be version-pinned (exact `=x.y.z`) and present in `Cargo.lock` before Buildroot build.
- Slint with `renderer-femtovg` has a pure-Rust dependency tree with no native C/C++ build scripts, making it compatible with Buildroot's cargo-package cross-compilation infrastructure without workarounds.

## Plan (incremental)

### Step 0 — Kernel config: enable AMD GPU and touchscreen drivers ✅ COMPLETE
**Requires:** Full ISO rebuild + reboot.

**Actions:**
- Add the following to `configs/kernel/steamdeck_defconfig`:
  - `CONFIG_DRM_AMDGPU=y` — AMD GPU driver (Steam Deck Van Gogh APU, RDNA2).
  - `CONFIG_DRM_AMD_DC=y` — AMD Display Core for modesetting.
  - `CONFIG_I2C=y` — I2C bus support.
  - `CONFIG_I2C_HID_CORE=y` — HID-over-I2C core.
  - `CONFIG_I2C_HID_ACPI=y` — ACPI enumeration for I2C HID devices.
  - `CONFIG_INPUT_TOUCHSCREEN=y` — Touchscreen input subsystem.
  - `CONFIG_HID_MULTITOUCH=y` — Generic multitouch HID driver (Deck touchscreen).
- Full ISO rebuild (`./scripts/build.sh`) + boot on Deck.

**Test script: `scripts/tests/test_gui_step0_kernel.sh`**

Automated checks:
- `/dev/dri/card*` exists
- `dmesg` contains `amdgpu` driver initialization messages
- Touchscreen input device found in `/proc/bus/input/devices`
- `hidd` service is running (no regression)
- `bluetoothd` service is running (no regression)
- Dump DRI device listing and touchscreen device info for diagnostics

Manual checks (printed as `MANUAL:` lines):
- Tap the touchscreen while running `cat /dev/input/<touchscreen_event>` — binary data appears

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step0_kernel.sh
```

**Complete when:**
- Test script exits 0 (all PASS).
- Manual touch tap produces input events.

### Step 1 — Buildroot packages: mesa3d, libinput, libxkbcommon ✅ COMPLETE
**Requires:** Full ISO rebuild + reboot.

**Actions:**
- Enable required Buildroot packages in `configs/buildroot/controlleros_defconfig`:
  - `BR2_PACKAGE_MESA3D=y`
  - `BR2_PACKAGE_MESA3D_GALLIUM_DRIVER_RADEONSI=y`
  - `BR2_PACKAGE_MESA3D_OPENGL_EGL=y`
  - `BR2_PACKAGE_MESA3D_OPENGL_ES=y`
  - `BR2_PACKAGE_LIBINPUT=y`
  - `BR2_PACKAGE_LIBXKBCOMMON=y`
- Full ISO rebuild + boot on Deck.

**Test script: `scripts/tests/test_gui_step1_packages.sh`**

Automated checks:
- `libEGL` and `libGLES` shared objects exist in `/usr/lib/`
- `radeonsi_dri.so` exists in `/usr/lib/dri/`
- `libinput` command is available and `libinput list-devices` includes a touchscreen
- `libxkbcommon` shared object exists in `/usr/lib/`
- `libdrm` shared object exists in `/usr/lib/`
- `/dev/dri/card*` still exists (Step 0 no regression)
- `hidd` and `bluetoothd` services still running (no regression)
- Dump `libinput list-devices` output for diagnostics

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step1_packages.sh
```

**Complete when:**
- Test script exits 0 (all PASS).

### Step 2 — Create `crates/gui/` crate skeleton and workspace integration ✅ COMPLETE
**Requires:** Loop 1 (host-only). No Deck testing yet.

**Actions:**
- Create `crates/gui/` as a binary crate (`controlleros-gui`).
- Add to workspace `Cargo.toml`.
- Add dependencies in `crates/gui/Cargo.toml`:
  ```toml
  [dependencies]
  slint = { version = "=1.15.0", default-features = false, features = [
      "backend-linuxkms-noseat",
      "renderer-femtovg",
      "compat-1-2",
  ] }
  zbus = { version = "=5.5.0", default-features = false, features = ["tokio"] }
  tokio = { version = "=1.44.1", features = ["rt", "macros", "process", "time"] }
  anyhow = "=1.0.97"
  tracing = "=0.1.41"
  tracing-subscriber = "=0.3.19"
  ```
  Note: For host-only development, consider a cargo feature flag (e.g., `deck`) that switches between LinuxKMS (for Deck) and default Slint backend (for host preview). Or just use the LinuxKMS features for all builds and test on Deck only.
- Create `crates/gui/ui/` directory.
- Create minimal `crates/gui/ui/main.slint` with `import { VerticalBox, Button } from "std-widgets.slint";` and a placeholder window containing a title `Text` and a single `Button`.
- Create `crates/gui/src/main.rs` that initializes the Slint window.
- Create `crates/gui/build.rs` with `slint_build::compile("ui/main.slint")`.
- Add `slint-build` as a build dependency.

**Validate on host (Loop 1):**
```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace
```

**Complete when:**
- `cargo check --workspace` passes with the new crate and all dependencies resolve.

### Step 3 — First render on Deck: placeholder GUI via DRM/KMS ✅ COMPLETE
**Requires:** Full rebuild (new Buildroot package `.mk`) + Loop 2 for subsequent iterations.

This is the critical integration milestone — get *anything* rendering on the Deck's screen via Slint + LinuxKMS.

**Actions:**
- Create `br2-external/package/controlleros-gui/controlleros-gui.mk`:
  - Follow the `controlleros-hidd.mk` pattern.
  - Set `CONTROLLEROS_GUI_SUBDIR = crates/gui`.
  - Add `CONTROLLEROS_GUI_DEPENDENCIES = mesa3d libinput libxkbcommon libdrm`.
- Create `br2-external/package/controlleros-gui/Config.in`.
- Update `br2-external/package/Config.in` to source the new package.
- Enable `BR2_PACKAGE_CONTROLLEROS_GUI=y` in defconfig.
- Update `scripts/dev_stage_payload.sh` to support `--gui <path>` argument.
- Build the image or cross-compile the binary and stage via Loop 2.
- Using `renderer-femtovg` (pure Rust, no native build scripts) — no special cross-compilation handling needed.

**Test script: `scripts/tests/test_gui_step3_render.sh`**

Automated checks:
- Locate `controlleros-gui` binary (dev path or `/usr/bin/`)
- Kill tty1 getty to free primary VT
- Launch GUI in background, wait 5 seconds
- GUI process is still alive (did not crash on startup)
- GUI log contains no `panic` / `fatal` errors
- `hidd` service still running (no interference)
- `bluetoothd` service still running
- Dump GUI startup log for diagnostics
- Kill GUI process at end

Manual checks (printed as `MANUAL:` lines):
- Deck screen shows the Slint placeholder window (title text + button)
- Display is landscape (1280×800) — if portrait, note `SLINT_KMS_ROTATION=270` needed
- Tapping the button produces visual press feedback
- Touch coordinates align with displayed elements

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --timeout-seconds 30 --shell-script test_gui_step3_render.sh
```

**Complete when:**
- Test script exits 0 (all PASS).
- Manual visual verification confirms rendering and touch.
- FemtoVG renderer confirmed working on Deck hardware.

### Step 4 — Full Slint UI layout with std-widgets
**Requires:** Loop 1 for layout iteration, then Loop 2 for Deck validation.

**Actions:**
- Replace `crates/gui/ui/main.slint` with the full UI layout using `std-widgets.slint` components:
  - `import { Button, ListView, GroupBox, VerticalBox, HorizontalBox, Spinner } from "std-widgets.slint";`
  - Top: `GroupBox` titled "ControllerOS — Bluetooth Devices" containing a `ListView`.
  - Each device row: `HorizontalBox` with `Text` (name), `Text` (MAC), `Text` (status indicator), `Button` ("Disconnect" or "Reconnect" based on state), `Button` ("Forget").
  - Bottom: `HorizontalBox` with `Button`s — "Restart Stack", "Restart", "Power Off".
  - Confirmation dialog: a modal overlay with message `Text`, `Button` ("Confirm"), `Button` ("Cancel").
  - Status bar: `Text` for operation feedback, `Spinner` while async operations are in progress.
- Define Slint structs and models:
  - `struct BtDeviceModel { name: string, address: string, connected: bool, obj_path: string }`
  - `property <[BtDeviceModel]> devices`
- Define callbacks: `disconnect(int)`, `reconnect(int)`, `forget(int)`, `restart-stack()`, `restart()`, `power-off()`, `confirm-action()`, `cancel-action()`.
- Target 1280×800 resolution. Touch targets minimum 48px height.
- Populate the device list with 2–3 hardcoded mock devices for visual validation.
- Wire callbacks to print to `tracing::info!()` so taps are visible in the log.

**Test script: `scripts/tests/test_gui_step4_layout.sh`**

Automated checks:
- GUI binary found
- Launch GUI, wait 5 seconds
- GUI process still alive
- No `panic` / `fatal` in log
- `hidd` still running

Manual checks (printed as `MANUAL:` lines):
- Full layout visible: device list with mock entries (name, MAC, status), action buttons per row, system buttons at bottom
- All text readable, all buttons large enough to tap
- Tap each button → check GUI log for corresponding `tracing::info!()` callback line
- Tap "Restart" → confirmation dialog appears → tap "Cancel" → dialog dismisses (repeat for Power Off, Restart Stack)
- If 3+ mock devices, ListView scrolls via touch drag

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --timeout-seconds 30 --shell-script test_gui_step4_layout.sh
```

**Complete when:**
- Test script exits 0 (all PASS).
- Manual verification confirms layout, button taps, confirmation dialog, and scrolling.

### Step 5 — BlueZ D-Bus client module
**Requires:** Loop 1 for code, then Loop 2 for Deck validation.

**Actions:**
- Create `crates/gui/src/bluez.rs` — async module using `zbus` for BlueZ interaction.
- Implement:
  - `list_paired_devices(connection: &zbus::Connection) -> Result<Vec<BtDevice>>`:
    - Use `ObjectManager` to get all objects under `/org/bluez`.
    - Filter for objects implementing `org.bluez.Device1`.
    - Read `Name`, `Address`, `Connected`, `Paired`, `Trusted` properties.
    - Return as `Vec<BtDevice>` (with D-Bus object path for later operations).
  - `disconnect_device(connection, obj_path) -> Result<()>`:
    - Call `Disconnect()` on `org.bluez.Device1` at `obj_path`.
  - `connect_device(connection, obj_path) -> Result<()>`:
    - Call `Connect()` on `org.bluez.Device1` at `obj_path`.
  - `remove_device(connection, adapter_path, device_path) -> Result<()>`:
    - Call `RemoveDevice(device_path)` on `org.bluez.Adapter1` at `adapter_path`.
- Define `BtDevice` struct: `name: String, address: String, connected: bool, paired: bool, trusted: bool, obj_path: String`.
- Add a CLI mode (`controlleros-gui --list-devices`) that connects to D-Bus, calls `list_paired_devices()`, and prints results to stdout. This enables scripted validation without the full GUI.

**Test script: `scripts/tests/test_gui_step5_bluez.sh`**

Prerequisites: at least one paired Bluetooth device.

Automated checks:
- `controlleros-gui --list-devices` exits successfully
- Every MAC address from `bluetoothctl devices` appears in `--list-devices` output
- For each device, `Connected` status from `--list-devices` matches `bluetoothctl info <MAC>`
- Dump both outputs for diagnostics

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step5_bluez.sh
```

**Complete when:**
- Test script exits 0 (all PASS).
- `--list-devices` output matches `bluetoothctl` data for all paired devices.

### Step 6 — Wire BlueZ backend to Slint UI
**Requires:** Loop 2 for Deck validation.

**Actions:**
- In `crates/gui/src/main.rs`, connect the Slint UI to the BlueZ D-Bus module:
  - Remove hardcoded mock devices from Step 4.
  - Initialize `zbus::Connection::system()` for D-Bus.
  - On startup: call `list_paired_devices()`, convert to `BtDeviceModel` Slint structs, populate the Slint `devices` model.
  - Set up a `tokio::time::interval` (every 2–3 seconds) to poll `list_paired_devices()` and update the Slint model. (D-Bus signal-based updates are a stretch goal; polling is simpler and sufficient for MVP.)
  - Wire callbacks using `slint::invoke_from_event_loop` or Slint's async callback support:
    - `disconnect(index)` → look up device by index, call `disconnect_device()`, refresh model.
    - `reconnect(index)` → look up device by index, call `connect_device()`, refresh model.
    - `forget(index)` → look up device by index, call `remove_device()`, refresh model.
  - Set `Spinner` visible during async operations; show success/error text in status bar.
- Handle the async runtime: Slint's event loop and tokio must coexist. Use `slint::spawn_local()` for dispatching async work from Slint callbacks, with a `tokio` runtime running in a background thread for D-Bus operations.

**Test script: `scripts/tests/test_gui_step6_wired.sh`**

Prerequisites: at least one paired Bluetooth device, host available for interaction.

Automated checks:
- Snapshot paired devices via `bluetoothctl devices` (fail if none)
- Launch GUI, wait 5 seconds
- GUI process still alive, no fatal errors in log
- `hidd` still running

Manual checks (printed as `MANUAL:` lines):
- GUI shows real paired devices (not mock data), matching `bluetoothctl` output
- **Disconnect**: tap "Disconnect" on a connected device → host loses controller, GUI shows disconnected, `bluetoothctl info <MAC>` confirms `Connected: no`
- **Reconnect**: tap "Reconnect" → host regains controller, GUI shows connected, `bluetoothctl info <MAC>` confirms `Connected: yes`
- **Forget**: disconnect first, tap "Forget" → device disappears from GUI, `bluetoothctl devices` no longer lists it, host must re-pair
- **Auto-refresh**: connect a host from the host side (not via GUI), wait 5s → GUI updates without any tap
- **Error handling**: try to reconnect a powered-off/out-of-range device → GUI shows error in status bar, does not crash

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --timeout-seconds 30 --shell-script test_gui_step6_wired.sh
```

**Complete when:**
- Test script exits 0 (all PASS).
- All manual disconnect/reconnect/forget/auto-refresh/error tests pass.

### Step 7 — System actions (restart, power off, restart stack)
**Requires:** Loop 2 for Deck validation.

**Actions:**
- Create `crates/gui/src/system.rs` — module for system commands.
- Implement:
  - `system_reboot() -> Result<()>` — execute `/sbin/reboot` via `tokio::process::Command`.
  - `system_poweroff() -> Result<()>` — execute `/sbin/poweroff` via `tokio::process::Command`.
  - `restart_stack() -> Result<()>` — execute init script stops/starts in sequence:
    1. `/etc/init.d/S45hidd stop`
    2. `/etc/init.d/S40bluetoothd stop`
    3. `/etc/init.d/S40bluetoothd start`
    4. `/etc/init.d/S45hidd start`
    5. Re-exec self via `std::process::Command::new(std::env::current_exe()?).exec()` (Unix exec replaces the process) or exit with a well-known code (e.g., 42) that the init script interprets as "restart me".
- Wire Slint callbacks:
  - "Restart Stack", "Restart", "Power Off" buttons set the confirmation dialog visible with appropriate message text.
  - `confirm-action` → invoke the pending system function.
  - `cancel-action` → hide the confirmation dialog.

**Test script: `scripts/tests/test_gui_step7_system.sh`**

Automated checks:
- GUI binary found, launch GUI, wait 5 seconds, process alive
- Init scripts `/etc/init.d/S45hidd` and `/etc/init.d/S40bluetoothd` exist and are executable
- `/sbin/reboot` and `/sbin/poweroff` are available
- Snapshot service state for diagnostics

Manual checks (printed as `MANUAL:` lines):
- **Cancel test**: tap each of "Restart", "Power Off", "Restart Stack" → confirmation dialog appears → tap "Cancel" → dialog dismisses, nothing happens
- **Restart Stack**: tap → confirm → GUI disappears briefly, services cycle, GUI reappears. Verify from tty2: `hidd` and `bluetoothd` running. Reconnect host, controller works. GUI device list repopulates.
- **Power Off**: tap → confirm → Deck shuts down cleanly
- **Restart**: boot Deck, run GUI, tap → confirm → Deck reboots

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --timeout-seconds 30 --shell-script test_gui_step7_system.sh
```

**Complete when:**
- Test script exits 0 (all PASS).
- All manual system action tests pass.

### Step 8 — Init script, auto-start, and display rotation
**Requires:** Full ISO rebuild + reboot.

**Actions:**
- Create `configs/init/S50gui`:
  - Starts after `S45hidd`.
  - `start)`: set `SLINT_KMS_ROTATION` env var if needed, launch `/usr/bin/controlleros-gui` in the background, redirect stdout/stderr to `/var/log/controlleros-gui.log`.
  - `stop)`: kill the `controlleros-gui` process.
  - `restart)`: stop then start.
- If display rotation is needed (determined in Step 3), create a udev rule for touchscreen coordinate calibration matrix and include it in post-build.
- Update `br2-external/board/controlleros/post-build.sh`:
  - Copy `S50gui` to `${TARGET_DIR}/etc/init.d/`.
  - Set executable permissions.
- Modify inittab handling: disable the `tty1` getty so the GUI has clean access to the primary VT. Keep `tty2` and `tty3` gettys for debugging.

**Test script: `scripts/tests/test_gui_step8_autostart.sh`**

Run after a fresh boot — do NOT launch GUI manually first.

Automated checks:
- `controlleros-gui` process is already running (auto-started by S50gui)
- `/etc/init.d/S50gui` exists and is executable
- `/var/log/controlleros-gui.log` exists and has content
- GUI log has no `panic` / `fatal` errors
- `hidd` and `bluetoothd` services are running
- `S50gui stop` → GUI process no longer running
- `S50gui start` → GUI process running again
- `S50gui restart` → GUI process running
- `tty2` getty process is running (debug terminal available)
- `tty3` getty process is running
- Dump GUI log tail for diagnostics

Manual checks (printed as `MANUAL:` lines):
- GUI appeared on screen automatically after boot, no login needed
- Display is landscape (1280×800), not portrait
- Tap buttons in each corner → taps register on correct elements (touch calibration)
- `chvt 2` → login prompt visible, `chvt 1` → GUI still visible
- Pair a host → controller input works while GUI is displayed

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step8_autostart.sh
```

**Complete when:**
- Test script exits 0 (all PASS).
- All manual visual/touch/VT-switching tests pass.

### Step 9 — Documentation and dev tooling
**Requires:** Loop 1 (host-only).

**Actions:**
- Create `docs/gui.md`:
  - Architecture: Slint + LinuxKMS-noseat backend, FemtoVG OpenGL renderer, BlueZ D-Bus integration via zbus, system command execution.
  - Slint UI structure: `std-widgets.slint` components used, callback/model bindings.
  - Touchscreen setup: kernel drivers, libinput integration, rotation/calibration details.
  - Framebuffer/DRM backend: how Slint renders via DRM/KMS on the Deck's AMD GPU (radeonsi + EGL).
  - Buildroot packages required and why.
  - Troubleshooting: display not showing, touch not working, D-Bus connection failures, renderer fallback options.
- Update `README.md`:
  - Document the GUI auto-start behavior.
  - Add `controlleros-gui` to the list of installed binaries.
  - Document how to stop/restart the GUI manually (`/etc/init.d/S50gui stop`).
- Finalize `scripts/dev_stage_payload.sh` `--gui` support (if not already done in Step 3).

**Complete when:**
- Docs exist with accurate content matching actual implementation.
- README reflects new binary and usage.

### Step 10 — End-to-end validation on Deck
**Requires:** Full ISO rebuild + reboot. This step performs the final acceptance test across all criteria on a clean boot.

**Actions:**
- Full ISO rebuild + boot on Deck.

**Test script: `scripts/tests/test_gui_step10_e2e.sh`**

Prerequisites: at least one previously paired host device, host available for interaction.

Automated checks:
- `controlleros-gui` process is running (auto-started)
- GUI log exists and has no fatal errors
- `hidd` and `bluetoothd` services are running
- Paired devices exist in `bluetoothctl devices`

Manual checks (printed as `MANUAL:` lines), organized by acceptance criterion:
- **A. GUI Launches on Boot**: GUI visible within 10s of boot, landscape, correct touch
- **B. Device List**: paired device in GUI with correct name/MAC/status; disconnect from host side → GUI updates within 5s; reconnect from host side → GUI updates
- **C. Disconnect / Reconnect**: tap Disconnect → host loses controller, GUI shows disconnected; tap Reconnect → host regains controller; controller test app confirms input works after reconnect
- **D. Forget Device**: disconnect, tap Forget → device gone from GUI and `bluetoothctl devices`; host must re-pair; re-pair to continue
- **E. Restart Stack**: tap → confirm → GUI cycles, services restart, device list repopulates, host can reconnect
- **F. Restart / Power Off**: tap Restart → confirm → Deck reboots → GUI auto-starts again; tap Power Off → confirm → Deck shuts down
- **G. No Regression**: `hidd` active, all MVP controls work on host (sticks, ABXY, D-pad, triggers, bumpers, Start/Back/Home, LS/RS); run 5 minutes with GUI touch + controller input active, no crashes/lag/corruption

**Run on Deck:**
```sh
controlleros-dev-run --base-url http://<DEV_MACHINE_IP>:8000 --shell-script test_gui_step10_e2e.sh
```

**Complete when:**
- Test script exits 0 (all PASS).
- All manual verification items confirmed on real hardware.
- No regressions in checkpoint 04 controller functionality.

---

## Acceptance Criteria Mapping
- A. GUI Launches on Boot: Steps 0, 1, 3, 8, 10
- B. Device List: Steps 4, 5, 6, 10
- C. Disconnect / Reconnect: Steps 5, 6, 10
- D. Forget Device: Steps 5, 6, 10
- E. Restart Stack: Steps 7, 8, 10
- F. Restart / Power Off: Steps 7, 10
- G. No Regression: Steps 0, 1, 3, 6, 8, 10
