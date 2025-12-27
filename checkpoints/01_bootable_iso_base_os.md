# Checkpoint 01 — Bootable ControllerOS Image (Base OS)

## Goal
Create a reproducible build that outputs a bootable image (ISO or bootable USB image) that boots on a Steam Deck into a minimal Linux shell.

## Scope (MVP)
- Boot to TTY shell is sufficient.
- No Bluetooth, no HID emulation, no controller mapping yet.

## Non-Goals
- Pairing UI
- Input mapping
- HID emulation
- Dual-boot installation (comes later)

---

## Required Repo Artifacts
- `README.md` with:
  - build prerequisites
  - build commands
  - how to boot on Steam Deck
- Build scripts:
  - `scripts/build.sh` (single entry point)
- Buildroot setup (preferred):
  - `buildroot/` (as a git submodule or pinned tarball extraction mechanism)
  - `configs/buildroot/controlleros_defconfig`
- Kernel configuration:
  - `configs/kernel/steamdeck_defconfig` (or fragment + documented base)
- Output:
  - `out/controlleros.iso` OR `out/controlleros.img` (gitignored)
- `docs/hw-matrix.md`:
  - record Steam Deck model and boot success (manually filled)

---

## Implementation Requirements
1. Build system is deterministic:
   - Buildroot revision pinned (commit hash or tarball checksum).
   - Kernel version pinned.
2. Image boots on Steam Deck to a shell prompt.
3. Root filesystem includes:
   - BusyBox core utilities
   - `cat`, `dmesg`, `uname`
4. Image includes a writable path for runtime state:
   - `mkdir -p /var/lib/controlleros` works and is writable.

---

## Testable Acceptance Criteria
### A. Build Test
- Running:
  - `./scripts/build.sh`
- Produces:
  - `out/controlleros.iso` or `out/controlleros.img`

### B. Boot Test (Steam Deck)
- Boot from USB (dock or USB-C drive).
- Successful if:
  - A console prompt appears.
  - `uname -a` works.
  - `cat /etc/os-release` prints ControllerOS identity.

### C. Writable State Test
- On the running OS:
  - `touch /var/lib/controlleros/testfile`
- Successful if the command exits 0.

---

## Definition of Done
- All required repo artifacts exist.
- Build is reproducible.
- Image boots on Steam Deck into a shell.
