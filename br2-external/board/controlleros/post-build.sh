#!/bin/sh
set -e

mkdir -p "${TARGET_DIR}/etc/bluetooth"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/bluez/main.conf" \
	"${TARGET_DIR}/etc/bluetooth/main.conf"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/bluez/input.conf" \
	"${TARGET_DIR}/etc/bluetooth/input.conf"

mkdir -p "${TARGET_DIR}/etc/init.d"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/init/S20dbus-prep" \
	"${TARGET_DIR}/etc/init.d/S20dbus-prep"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/init/S30dbus" \
	"${TARGET_DIR}/etc/init.d/S30dbus"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/init/S35bluetooth-storage" \
	"${TARGET_DIR}/etc/init.d/S35bluetooth-storage"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/init/S40bluetoothd" \
	"${TARGET_DIR}/etc/init.d/S40bluetoothd"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/init/S41bluetooth-power" \
	"${TARGET_DIR}/etc/init.d/S41bluetooth-power"
chmod 0755 "${TARGET_DIR}/etc/init.d/S20dbus-prep" \
	"${TARGET_DIR}/etc/init.d/S30dbus" \
	"${TARGET_DIR}/etc/init.d/S35bluetooth-storage" \
	"${TARGET_DIR}/etc/init.d/S40bluetoothd" \
	"${TARGET_DIR}/etc/init.d/S41bluetooth-power"

mkdir -p "${TARGET_DIR}/usr/bin"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/dev/controlleros-dev-update" \
	"${TARGET_DIR}/usr/bin/controlleros-dev-update"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/dev/controlleros-dev-list" \
	"${TARGET_DIR}/usr/bin/controlleros-dev-list"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/dev/controlleros-dev-run" \
	"${TARGET_DIR}/usr/bin/controlleros-dev-run"
chmod 0755 "${TARGET_DIR}/usr/bin/controlleros-dev-update" \
	"${TARGET_DIR}/usr/bin/controlleros-dev-list" \
	"${TARGET_DIR}/usr/bin/controlleros-dev-run"

mkdir -p "${TARGET_DIR}/etc/controlleros"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/hid/hid.toml" \
	"${TARGET_DIR}/etc/controlleros/hid.toml"

# Ensure additional virtual console gettys for multi-terminal debugging.
INITTAB="${TARGET_DIR}/etc/inittab"
if [ -f "${INITTAB}" ]; then
	if ! grep -q '^tty2::respawn:/sbin/getty -L  tty2 0 vt100 # GENERIC_SERIAL$' "${INITTAB}"; then
		printf '%s\n' 'tty2::respawn:/sbin/getty -L  tty2 0 vt100 # GENERIC_SERIAL' >> "${INITTAB}"
	fi
	if ! grep -q '^tty3::respawn:/sbin/getty -L  tty3 0 vt100 # GENERIC_SERIAL$' "${INITTAB}"; then
		printf '%s\n' 'tty3::respawn:/sbin/getty -L  tty3 0 vt100 # GENERIC_SERIAL' >> "${INITTAB}"
	fi
fi
