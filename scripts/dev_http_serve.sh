#!/bin/sh

set -eu

SERVE_DIR="out/dev-payload"
PORT="8000"
BIND_ADDR="0.0.0.0"

usage() {
	echo "Usage: $0 [--dir <path>] [--port <port>] [--bind <addr>]"
	echo "Defaults:"
	echo "  --dir  out/dev-payload"
	echo "  --port 8000"
	echo "  --bind 0.0.0.0"
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--dir)
			[ "$#" -ge 2 ] || {
				echo "error: --dir requires a value" >&2
				usage >&2
				exit 1
			}
			SERVE_DIR="$2"
			shift 2
			;;
		--port)
			[ "$#" -ge 2 ] || {
				echo "error: --port requires a value" >&2
				usage >&2
				exit 1
			}
			PORT="$2"
			shift 2
			;;
		--bind)
			[ "$#" -ge 2 ] || {
				echo "error: --bind requires a value" >&2
				usage >&2
				exit 1
			}
			BIND_ADDR="$2"
			shift 2
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "error: unknown argument: $1" >&2
			usage >&2
			exit 1
			;;
	esac
done

[ -d "$SERVE_DIR" ] || {
	echo "error: serve directory not found: $SERVE_DIR" >&2
	exit 1
}

echo "Serving $SERVE_DIR on http://$BIND_ADDR:$PORT"
exec python3 -m http.server "$PORT" --bind "$BIND_ADDR" --directory "$SERVE_DIR"
