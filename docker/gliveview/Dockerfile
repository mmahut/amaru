FROM debian:bookworm-slim

# Install required dependencies
RUN apt-get update && apt-get install -y \
    bash \
    curl \
    jq \
    git \
    bc \
    gawk \
    netcat-traditional \
    procps \
    iproute2 \
    && rm -rf /var/lib/apt/lists/*

# Create work directory
WORKDIR /opt/cardano

# Download gLiveView and its env file from guild-operators
RUN git clone --depth 1 https://github.com/cardano-community/guild-operators.git \
    && cp guild-operators/scripts/cnode-helper-scripts/gLiveView.sh . \
    && cp guild-operators/scripts/cnode-helper-scripts/env . \
    && rm -rf guild-operators \
    && chmod +x gLiveView.sh

# Stub binaries: gLiveView requires cardano-cli and cardano-node to be present.
# Both report Amaru's version (0.1.2) to match the running version from Prometheus.
# cardano-cli: only used for version queries.
# cardano-node: for version queries it prints the version string; for all other
# invocations (e.g. --port 6000 from entrypoint) it sleeps indefinitely so that
# gLiveView's `pgrep -fn "cardano-node.*.port <PORT>"` check finds a live process.
RUN printf '#!/bin/sh\n[ "$1" = "version" ] && echo "cardano-cli 0.1.2 - linux-x86_64" && exit 0\nexit 1\n' \
        > /usr/local/bin/cardano-cli \
    && printf '#!/bin/sh\n[ "$1" = "version" ] && echo "cardano-node 0.1.2 - linux-x86_64" && exit 0\nsleep infinity\n' \
        > /usr/local/bin/cardano-node \
    && chmod +x /usr/local/bin/cardano-cli /usr/local/bin/cardano-node

# Disable the strict version check in env so the 0.1.2 version string is accepted.
# versionCheckNode() short-circuits to success when STRICT_VERSION_CHECK=N.
# Also disable the auto-update check so the container doesn't prompt at startup.
# Also stub out checkNodeVersion to suppress the deployed/running rev mismatch
# warning (Amaru doesn't emit a git revision in Prometheus metrics).
RUN sed -i 's/^#STRICT_VERSION_CHECK=.*/STRICT_VERSION_CHECK="N"/' /opt/cardano/env \
    && sed -i 's/^#UPDATE_CHECK=.*/UPDATE_CHECK="N"/' /opt/cardano/env \
    && sed -i 's/^checkNodeVersion$/checkNodeVersion() { return 0; }/' /opt/cardano/gLiveView.sh

# Entrypoint downloads network genesis/config files at runtime then launches gLiveView
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENV NETWORK="preprod" \
    BLOCKLOG_DIR="/opt/cardano/blocklog" \
    PROM_HOST="host.docker.internal" \
    PROM_PORT="8889"

RUN mkdir -p "${BLOCKLOG_DIR}" /opt/logs

ENTRYPOINT ["/entrypoint.sh"]
