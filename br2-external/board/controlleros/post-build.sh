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
