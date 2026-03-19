#!/bin/sh
set -e
exec cargo run -p controlleros-gui --no-default-features --features desktop
