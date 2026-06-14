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
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/init/S45hidd" \
	"${TARGET_DIR}/etc/init.d/S45hidd"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/init/S01gui" \
	"${TARGET_DIR}/etc/init.d/S01gui"
# Remove old S50gui from previous builds (renamed to S01gui)
rm -f "${TARGET_DIR}/etc/init.d/S50gui"
chmod 0755 "${TARGET_DIR}/etc/init.d/S20dbus-prep" \
	"${TARGET_DIR}/etc/init.d/S30dbus" \
	"${TARGET_DIR}/etc/init.d/S35bluetooth-storage" \
	"${TARGET_DIR}/etc/init.d/S40bluetoothd" \
	"${TARGET_DIR}/etc/init.d/S41bluetooth-power" \
	"${TARGET_DIR}/etc/init.d/S45hidd" \
	"${TARGET_DIR}/etc/init.d/S01gui"

mkdir -p "${TARGET_DIR}/usr/bin"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/dev/cosd-update" \
	"${TARGET_DIR}/usr/bin/cosd-update"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/dev/cosd-list" \
	"${TARGET_DIR}/usr/bin/cosd-list"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/dev/cosd-run" \
	"${TARGET_DIR}/usr/bin/cosd-run"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/dev/cosd-debug" \
	"${TARGET_DIR}/usr/bin/cosd-debug"
chmod 0755 "${TARGET_DIR}/usr/bin/cosd-update" \
	"${TARGET_DIR}/usr/bin/cosd-list" \
	"${TARGET_DIR}/usr/bin/cosd-run" \
	"${TARGET_DIR}/usr/bin/cosd-debug"

# Inject build-host IP as the default --base-url in cosd-run and cosd-update.
BUILD_IP="$(ip route get 1 2>/dev/null \
	| awk 'NR==1{for(i=1;i<=NF;i++){if($i=="src"){print $(i+1);exit}}}')" || true
if [ -n "${BUILD_IP:-}" ]; then
	BAKED="http://${BUILD_IP}:8000"
else
	BAKED=""
fi
sed -i "s|@@BUILD_BASE_URL@@|${BAKED}|g" \
	"${TARGET_DIR}/usr/bin/cosd-run" \
	"${TARGET_DIR}/usr/bin/cosd-update"

mkdir -p "${TARGET_DIR}/etc/controlleros"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/hid/hid.toml" \
	"${TARGET_DIR}/etc/controlleros/hid.toml"

mkdir -p "${TARGET_DIR}/etc/controlleros/mapping"
cp -f "${BR2_EXTERNAL_CONTROLLEROS_PATH}/../configs/mapping/xbox.toml" \
	"${TARGET_DIR}/etc/controlleros/mapping/xbox.toml"

# Disable tty1 getty so the GUI has clean access to the primary VT.
# Keep tty2 and tty3 gettys for debugging.
INITTAB="${TARGET_DIR}/etc/inittab"
if [ -f "${INITTAB}" ]; then
	# Comment out any tty1 getty lines
	sed -i 's/^.*getty.*tty1/#&/' "${INITTAB}"
	if ! grep -q '^tty2::respawn:/sbin/getty -L  tty2 0 vt100 # GENERIC_SERIAL$' "${INITTAB}"; then
		printf '%s\n' 'tty2::respawn:/sbin/getty -L  tty2 0 vt100 # GENERIC_SERIAL' >> "${INITTAB}"
	fi
	if ! grep -q '^tty3::respawn:/sbin/getty -L  tty3 0 vt100 # GENERIC_SERIAL$' "${INITTAB}"; then
		printf '%s\n' 'tty3::respawn:/sbin/getty -L  tty3 0 vt100 # GENERIC_SERIAL' >> "${INITTAB}"
	fi
fi
