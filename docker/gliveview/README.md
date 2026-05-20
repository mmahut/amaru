# GLiveView

Docker setup for running [GLiveView](https://cardano-community.github.io/guild-operators/Scripts/gliveview/) alongside [Amaru](https://github.com/pragma-org/amaru).

GLiveView relies on prometheus metrics exposed by Amaru to display live chain and peer metrics.

## Prerequisites

- A running Amaru node with an accessible socket file and Prometheus metrics endpoint
- Docker installed

## Usage

Build and run:

```bash
docker build --no-cache -t gliveview . && docker run --rm -it gliveview
```

## Environment Variables

Pass with `-e VAR=value` to override defaults.

| Variable | Default | Description |
|---|---|---|
| `NETWORK` | `preprod` | Target network (`mainnet`, `preprod`, `preview`) — used to fetch genesis files |
| `PROM_HOST` | `host.docker.internal` | Host running Amaru's Prometheus metrics |
| `PROM_PORT` | `8889` | Prometheus port exposed by the OTLP collector |
| `BLOCKLOG_DIR` | `/opt/cardano/blocklog` | Block log storage |
