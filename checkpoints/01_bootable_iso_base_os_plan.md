# Checkpoint 01 Plan — Bootable ControllerOS Image (Base OS)

Pinned Buildroot version: **2025.11** (submodule).

## Plan (incremental)

### Step 1 — Pin Buildroot 2025.11 as submodule
- [x] Done
**Complete when:**
- `buildroot/` exists as a git submodule pointing to the Buildroot 2025.11 tag or commit.
- The exact tag/commit is documented in `README.md`.

### Step 2 — Set up out-of-tree build output
- [x] Done
**Complete when:**
- `out/` is gitignored.
- Buildroot output is configured to live under `out/buildroot` (via `O=...`).
- The build layout is documented in `README.md`.

### Step 3 — Create Buildroot defconfig
- [x] Done
**Complete when:**
- `configs/buildroot/controlleros_defconfig` exists and targets x86_64.
- The defconfig enables:
  - Linux kernel build.
  - BusyBox core utilities (including `cat`, `dmesg`, `uname`).
  - ISO image generation (hybrid ISO acceptable for USB boot).
- `BR2_LINUX_KERNEL_CUSTOM_CONFIG_FILE` points to `configs/kernel/steamdeck_defconfig`.

### Step 4 — Create kernel defconfig
- [x] Done
**Complete when:**
- `configs/kernel/steamdeck_defconfig` exists.
- Kernel boots to a TTY shell on Steam Deck hardware.
- Required minimal support for booting a live ISO is present (x86_64, EFI/bootloader as needed).

### Step 5 — Rootfs identity and writable state path
- [x] Done
**Complete when:**
- Target image contains `/etc/os-release` with ControllerOS identity.
- `/var/lib/controlleros` exists and is writable at runtime.

### Step 6 — Single entry build script
- [x] Done
**Complete when:**
- `scripts/build.sh` exists and is executable.
- Running `./scripts/build.sh` produces `out/controlleros.iso` (or `.img`).
- Build script uses the defconfig and out-of-tree build output.

### Step 7 — Documentation and hardware matrix
- [x] Done
**Complete when:**
- `README.md` documents prerequisites, build steps, and Steam Deck boot steps.
- `docs/hw-matrix.md` exists with a placeholder entry for Steam Deck model and boot results.

---

## Acceptance Criteria Mapping
- Build Test: Steps 1, 2, 3, 6
- Boot Test: Steps 3, 4, 5, 7
- Writable State Test: Step 5

## Progress Updates
Update this plan by marking steps as **Done** when complete and recording any deviations.
