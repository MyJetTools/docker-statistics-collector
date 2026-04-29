# docker-statistics-collector

A lightweight service that connects to the Docker Engine API, collects container
information and runtime statistics (CPU, memory), and aggregates Prometheus
metrics exposed by containers running under `docker compose`.

## Workspace layout

- [docker-sdk/](docker-sdk/) — a minimal async client for the Docker Engine
  HTTP API (list containers, fetch container stats).
- [docker-statistics-collector/](docker-statistics-collector/) — the collector
  service: background timers, in-memory caches, an HTTP/Swagger API, and an
  MCP (Model Context Protocol) endpoint at `/mcp`.

## How it works

Two timers run every 5 seconds:

1. **Containers sync** — calls the Docker Engine API to list containers and,
   for each running container, fetches runtime stats. CPU usage, memory usage,
   available memory, and memory limit are stored in an in-memory cache. See
   [sync_containers_info_timer.rs](docker-statistics-collector/src/timers/sync_containers_info_timer.rs).
2. **Metrics sync** — for every container that carries the
   `com.docker.compose.service` label, the collector scrapes
   `http://<service_name>:<metrics_port>/metrics`. If the response looks like
   Prometheus text format, an `app="<service_name>"` label is injected into
   every metric line and the payload is stored in the metrics cache. See
   [sync_metrics_endpoints_timer.rs](docker-statistics-collector/src/timers/sync_metrics_endpoints_timer.rs).

The HTTP server listens on port `8000` and exposes a Swagger UI. See
[start_up.rs](docker-statistics-collector/src/http/start_up.rs).

## HTTP endpoints

- `GET /containers` — list of containers with their state, image, ports and
  usage snapshot.
- `GET /containers/running` — only running containers.
- `GET /containers/logs` — container logs.
- `GET /metrics` — aggregated Prometheus metrics from all scraped services.
- `GET /metrics/services` — list of services that have collected metrics.
- `GET /metrics/service` — metrics for a single service.

Controller sources: [containers_controller/](docker-statistics-collector/src/http/controllers/containers_controller/),
[metrics_controller/](docker-statistics-collector/src/http/controllers/metrics_controller/).

## Configuration

Settings are read from `~/.docker-statistics-collector` (YAML). See
[settings.rs](docker-statistics-collector/src/settings.rs).

| Field                        | Type           | Description                                                                       |
| ---------------------------- | -------------- | --------------------------------------------------------------------------------- |
| `docker_url`                 | `string`       | Docker Engine API endpoint. TCP: `http://localhost:2375`. Unix socket: `http+unix://var/run/docker.sock`. |
| `metrics_port`               | `u16`          | Port on which each service exposes its Prometheus `/metrics` endpoint.            |
| `disable_metics_collecting`  | `bool?`        | If `true`, the metrics scraping timer is a no-op.                                 |
| `services_to_ignore`         | `list<string>?`| Optional. `com.docker.compose.service` values to skip during scraping.            |
| `peers`                      | `list<string>?`| Optional. Base URLs of peer collector instances to federate with (see below).     |
| `peers_sync_interval_secs`   | `u64?`         | Optional. Interval for polling peers. Default `5`.                                |
| `peers_request_timeout_secs` | `u64?`         | Optional. Per-peer request timeout. Default `5`.                                  |

Example:

```yaml
docker_url: http://localhost:2375
metrics_port: 9091
disable_metics_collecting: false
services_to_ignore:
  - nginx
  - redis
peers:
  - http://collector-b:8000
  - http://collector-c:8000
peers_sync_interval_secs: 5
peers_request_timeout_secs: 5
```

The HTTP listen port (`8000`) is hardcoded in
[start_up.rs](docker-statistics-collector/src/http/start_up.rs).

The optional `ENV_INFO` environment variable is surfaced via the app context.

## Build and run

Locally:

```bash
cargo build --release
./target/release/docker-statistics-collector
```

In Docker — see [Dockerfile](Dockerfile):

```bash
cargo build --release
docker build -t docker-statistics-collector .
docker run --rm \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v $HOME/.docker-statistics-collector:/root/.docker-statistics-collector \
  -p 8000:8000 \
  docker-statistics-collector
```

Minimal `docker-compose.yaml`:

```yaml
services:
  docker-statistics-collector:
    image: ghcr.io/myjettools/docker-statistics-collector:0.2.5
    container_name: docker-statistics-collector
    restart: always
    environment:
    - ENV_INFO
    volumes:
    - /var/run/docker.sock:/var/run/docker.sock
    - ./.docker-statistics-collector:/root/.docker-statistics-collector:ro
```

To use a Unix socket, set `docker_url: http+unix://var/run/docker.sock` and
keep the `/var/run/docker.sock` bind-mount (read-only is enough). For TCP,
drop the socket mount and point `docker_url` at the daemon's HTTP endpoint.

## Federation

When `peers` is set, a third timer (cadence `peers_sync_interval_secs`) polls
each peer's `GET /api/containers/local` endpoint and stores the response in an
in-memory peer cache. The instance identifier of each peer comes from the
peer's `ENV_INFO` environment variable.

The merging happens at read time:

- `GET /api/containers` and `GET /api/containers/running` now return the union
  of local containers and every peer's last successful snapshot. Each item
  carries an `instance` field naming the source.
- `GET /api/containers/logs?id=...&lines_number=...` auto-resolves: if the id
  belongs to a peer it proxies the request to that peer's local logs endpoint.
- The `/mcp` tools (`find_containers`, `get_container_logs`) behave the same
  way — searches span all instances and log retrieval auto-routes.
- `GET /api/containers/local` is the **peer-facing** endpoint; it returns only
  this instance's data and never recurses into peers, so two collectors can
  reciprocally peer each other safely.
- `/metrics` aggregation across peers is **not** federated in this version.

## MCP endpoint

The collector exposes an MCP (Model Context Protocol) endpoint at `/mcp` on the
same port (`8000`). It uses MCP **Streamable HTTP** transport and is built on
the official [`rmcp`](https://crates.io/crates/rmcp) SDK. Two tools are
exposed:

- `find_containers(phrase, only_running?)` — case-insensitive substring match
  against container id, names, image, and labels. Returns id, names, image,
  state, status, ports, labels, the `com.docker.compose.service` value, and the
  latest CPU/memory snapshot from the cache.
- `get_container_logs(container_id, tail?)` — combined stdout/stderr tail
  (defaults to 200 lines), with Docker's multiplexed framing stripped.

Implementation: see [mcp/server.rs](docker-statistics-collector/src/mcp/server.rs)
and [mcp/middleware.rs](docker-statistics-collector/src/mcp/middleware.rs);
the middleware is registered in
[start_up.rs](docker-statistics-collector/src/http/start_up.rs).

Register with an MCP-aware client that supports streamable-HTTP transport:

```json
{
  "mcpServers": {
    "docker-statistics": {
      "type": "http",
      "url": "http://localhost:8000/mcp"
    }
  }
}
```

## Requirements

- Rust (edition 2021).
- Access to a Docker Engine API endpoint.
- Scraped services must expose Prometheus text-format metrics on
  `/metrics` at the configured `metrics_port`.
