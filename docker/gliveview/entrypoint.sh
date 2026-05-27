#!/bin/bash
set -euo pipefail

NETWORK="${NETWORK:-preprod}"
CONFIGS_URL="https://book.world.dev.cardano.org/environments"
FILES_DIR="/opt/cardano/files"

case "$NETWORK" in
  mainnet|preprod|preview) NET_PATH="$NETWORK" ;;
  *) echo "ERROR: Unknown NETWORK '$NETWORK'. Valid values: mainnet, preprod, preview" && exit 1 ;;
esac

echo "Fetching $NETWORK node configuration..."
mkdir -p "$FILES_DIR"
for f in config.json byron-genesis.json shelley-genesis.json alonzo-genesis.json conway-genesis.json; do
  curl -sSfL "$CONFIGS_URL/$NET_PATH/$f" -o "$FILES_DIR/$f"
done

export CONFIG="$FILES_DIR/config.json"

# gLiveView runs `pgrep -fn "cardano-node.*.port <CNODE_PORT>"` each iteration
# and increments a fail counter when no matching process is found, eventually
# exiting with "COULD NOT CONNECT". Start a background stub process whose
# command line matches the pattern so the PID check always succeeds.
CNODE_PORT="${CNODE_PORT:-6000}"
/usr/local/bin/cardano-node --port "$CNODE_PORT" &

# The OTLP collector's Prometheus exporter uses Go's 'g' float format, which
# produces scientific notation for large numbers (e.g. 7.6137225e+07). bash
# arithmetic inside gLiveView cannot parse that. Run a proxy on localhost:12798
# that fetches from the real endpoint and converts 'e+' notation to plain integers.
REAL_PROM_HOST="${PROM_HOST:-host.docker.internal}"
REAL_PROM_PORT="${PROM_PORT:-8889}"
PROXY_PORT=12798
METRICS_FILE=/tmp/metrics_formatted

(
    while true; do
        curl -s "http://${REAL_PROM_HOST}:${REAL_PROM_PORT}/metrics" \
            | awk '/^[^#]/ && NF > 0 && $NF ~ /[0-9]e\+[0-9]/ { $NF = sprintf("%.0f", $NF + 0) } { print }' \
            > "${METRICS_FILE}.tmp" \
            && mv "${METRICS_FILE}.tmp" "$METRICS_FILE"
        sleep 2
    done
) &

(
    while true; do
        if [[ -f "$METRICS_FILE" ]]; then
            {
                printf "HTTP/1.0 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\n\r\n"
                cat "$METRICS_FILE"
            } | nc -l -p "$PROXY_PORT" -q 1 >/dev/null 2>&1 || true
        else
            sleep 0.5
        fi
    done
) &

export PROM_HOST=127.0.0.1
export PROM_PORT=$PROXY_PORT

exec /opt/cardano/gLiveView.sh
