#!/bin/sh

set -eu

SERVE_DIR="out/dev-payload"
LOGS_DIR="out/dev-logs"
PORT="8000"
BIND_ADDR="0.0.0.0"
LOG_ENDPOINT="/logs"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
SERVER_PY="$SCRIPT_DIR/dev_http_server.py"

usage() {
	echo "Usage: $0 [--dir <path>] [--logs-dir <path>] [--port <port>] [--bind <addr>] [--log-endpoint <path>]"
	echo "Defaults:"
	echo "  --dir  out/dev-payload"
	echo "  --logs-dir out/dev-logs"
	echo "  --port 8000"
	echo "  --bind 0.0.0.0"
	echo "  --log-endpoint /logs"
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
		--logs-dir)
			[ "$#" -ge 2 ] || {
				echo "error: --logs-dir requires a value" >&2
				usage >&2
				exit 1
			}
			LOGS_DIR="$2"
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
		--log-endpoint)
			[ "$#" -ge 2 ] || {
				echo "error: --log-endpoint requires a value" >&2
				usage >&2
				exit 1
			}
			LOG_ENDPOINT="$2"
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

[ -f "$SERVER_PY" ] || {
	echo "error: python server not found: $SERVER_PY" >&2
	exit 1
}

mkdir -p "$LOGS_DIR"

echo "Serving payload dir: $SERVE_DIR"
echo "Receiving logs at: http://$BIND_ADDR:$PORT$LOG_ENDPOINT"
echo "Storing logs in: $LOGS_DIR"
exec python3 "$SERVER_PY" \
	--dir "$SERVE_DIR" \
	--logs-dir "$LOGS_DIR" \
	--port "$PORT" \
	--bind "$BIND_ADDR" \
	--log-endpoint "$LOG_ENDPOINT"
