# Checkpoint 05 — Installable Dual-Boot + Release Artifacts

## Goal
Provide a repeatable way to install ControllerOS alongside SteamOS on a Steam Deck, with a reliable boot menu entry and a release workflow that produces versioned images + checksums.

## Scope (MVP)
- Manual installation steps are acceptable (documented).
- Must not break SteamOS boot.
- Must provide uninstall/rollback.

---

## Required Repo Artifacts
- Docs:
  - `docs/install.md` (step-by-step)
  - `docs/uninstall.md`
  - `docs/boot.md` (boot entry mechanism)
  - `docs/partitioning.md` (recommended layout)
- Scripts:
  - `scripts/release.sh` (build + checksums)
  - `scripts/install_helper.sh` (optional; if included, must be conservative)
- Outputs:
  - `out/controlleros-<version>.iso` (or `.img`)
  - `out/SHA256SUMS`

---

## Implementation Requirements
1. Provide a recommended partition layout that includes:
   - root partition
   - persistent state partition (for `/var/lib/bluetooth` at minimum)
2. Add boot entry named `ControllerOS` using a Steam Deck compatible bootloader approach.
3. Release artifacts are versioned and checksummed.
4. Uninstall steps restore original boot behavior.

---

## Testable Acceptance Criteria
### A. Install Test
- Following `docs/install.md` on a stock Steam Deck:
  - ControllerOS is installed to internal SSD (preferred) OR documented supported target.
- Successful if:
  - SteamOS still boots
  - ControllerOS boots from boot menu

### B. End-to-End Controller Test
- Under installed ControllerOS:
  - Pair host (or reconnect)
  - Controller works (checkpoint 04 behavior)

### C. Reboot/Persistence
- Reboot ControllerOS:
  - pairing still present
  - controller still works

### D. Uninstall Test
- Following `docs/uninstall.md`:
  - ControllerOS boot entry removed
  - SteamOS boots normally

### E. Release Workflow
- Run:
  - `./scripts/release.sh`
- Successful if:
  - versioned image produced
  - `sha256sum -c out/SHA256SUMS` passes

---

## Definition of Done
- Repo builds a release image, installable alongside SteamOS on any standard Steam Deck, and the installed ControllerOS provides the MVP Bluetooth controller functionality.
