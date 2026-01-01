# ControllerOS

## Buildroot pin

This repo vendors Buildroot as a submodule pinned to tag `2025.11` (commit `08d71521d3`).

## Build prerequisites

- Standard Buildroot host dependencies (see Buildroot manual for your distro)
- `git` with submodule support
- `make`, `gcc`, `g++`, `python3`, `tar`, `xz`, `rsync`

## Build output layout

All Buildroot output is placed under `out/buildroot` using the out-of-tree build option (`O=out/buildroot`).
Generated images are copied to `out/` for convenience.

## Build (manual)

1) Configure Buildroot (defconfig):
```bash
make -C buildroot O=../out/buildroot BR2_EXTERNAL=../br2-external \
  BR2_DEFCONFIG=../configs/buildroot/controlleros_defconfig defconfig
```

2) Build:
```bash
make -C buildroot O=../out/buildroot BR2_EXTERNAL=../br2-external
```

3) Copy/rename ISO for Ventoy:
```bash
cp out/buildroot/images/rootfs.iso9660 out/buildroot/images/controlleros.iso
```

## Boot on Steam Deck (Ventoy)

1) Create a Ventoy USB stick.
2) Copy `out/buildroot/images/controlleros.iso` to the Ventoy USB root.
3) Power off the Steam Deck.
4) Hold Volume Down and press Power to open the boot manager.
5) Select the Ventoy USB device.
6) Use Ventoy's GRUB2 mode and select `controlleros.iso`.
7) Successful boot shows a ControllerOS login prompt.
