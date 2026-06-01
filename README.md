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
   available memory, memory limit, the number of open file descriptors and the
   `nofile` limit are stored in an in-memory cache. See
   [sync_containers_info_timer.rs](docker-statistics-collector/src/timers/sync_containers_info_timer.rs).
   File-descriptor stats require the host `/proc` to be mounted — see
   [File descriptor statistics](#file-descriptor-statistics).
2. **Metrics sync** — for every container that carries the
   `com.docker.compose.service` label, the collector scrapes
   `http://<service_name>:<metrics_port>/metrics`. If the response looks like
   Prometheus text format, an `app="<service_name>"` label is injected into
   every metric line and the payload is stored in the metrics cache. See
   [sync_metrics_endpoints_timer.rs](docker-statistics-collector/src/timers/sync_metrics_endpoints_timer.rs).

The HTTP server listens on port `8000` and exposes a Swagger UI. See
[start_up.rs](docker-statistics-collector/src/http/start_up.rs).

## HTTP endpoints

- `GET /containers` — list of containers with their state, image, ports, volumes
  and usage snapshot (CPU, memory, and a `files` block with open file descriptors
  and the `nofile` limit).
- `GET /containers/running` — only running containers.
- `GET /containers/logs` — container logs.
- `GET /containers/processes` — every process inside one container with its
  open file descriptors and `nofile` limit (federation-aware, like logs).
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
| `host_proc_path`             | `string?`      | Optional. Path inside the collector container where the host `/proc` is mounted. Used to read per-container open file descriptors and `nofile` limits. Default `/host/proc`. See [File descriptor statistics](#file-descriptor-statistics). |
| `host_root_path`             | `string?`      | Optional. Path inside the collector container where the host root filesystem is bind-mounted (`-v /:/host/root:ro`). Used to `statvfs` each host mount point for physical-disk usage. Default `/host/root`. See [Host disk statistics](#host-disk-statistics). |
| `ignore_disks`               | `list<string>?`| Optional. Physical disks to hide from host disk stats. Each entry matches either the block device (`/dev/sda15`) or the mount point (`/boot/efi`). |

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
# Path where the host `/proc` is mounted inside the collector container.
# Default is `/host/proc` — see "File descriptor statistics" below for the
# required `-v /proc:/host/proc:ro` volume mount.
host_proc_path: /host/proc
# Path where the host root filesystem is bind-mounted inside the collector
# container. Default is `/host/root` — see "Host disk statistics" below for the
# required `-v /:/host/root:ro` volume mount.
host_root_path: /host/root
# Disks to hide from host disk stats — matched by device or mount point.
ignore_disks:
  - /boot/efi
  - /dev/sdf
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
  -v /proc:/host/proc:ro \
  -v /:/host/root:ro \
  -v $HOME/.docker-statistics-collector:/root/.docker-statistics-collector \
  -p 8000:8000 \
  docker-statistics-collector
# ^ -v /proc:/host/proc:ro  — REQUIRED for the per-container file-descriptor
#   stats (Files: open/limit, leak graph, Processes dialog). Without it the
#   `files` block in the API and the UI shows N/A.
# ^ -v /:/host/root:ro      — REQUIRED for host physical-disk usage (which
#   disks exist on the host and how much is used). Without it the `disks`
#   array of each host entry stays empty. See Host disk statistics.
```

The `-v /proc:/host/proc:ro` mount powers the per-container file-descriptor
stats — see [File descriptor statistics](#file-descriptor-statistics). The
`-v /:/host/root:ro` mount powers host physical-disk usage — see
[Host disk statistics](#host-disk-statistics).

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
    # REQUIRED for the per-container file-descriptor stats (Files: open/limit,
    # leak graph, Processes dialog). Without this the `files` block is N/A.
    - /proc:/host/proc:ro
    # REQUIRED for host physical-disk usage (which disks exist + how much used).
    # Read-only, recursive — submounts on separate disks are measured too.
    - /:/host/root:ro
    - ./.docker-statistics-collector:/root/.docker-statistics-collector:ro
```

To use a Unix socket, set `docker_url: http+unix://var/run/docker.sock` and
keep the `/var/run/docker.sock` bind-mount (read-only is enough). For TCP,
drop the socket mount and point `docker_url` at the daemon's HTTP endpoint.

## File descriptor statistics

For every running container the collector reports **how many file descriptors
its main process currently has open** and that process's **`nofile` soft
limit** (`RLIMIT_NOFILE`). The "main process" is the one started by the image
`ENTRYPOINT`/`CMD` — PID 1 inside the container. `RLIMIT_NOFILE` is enforced
per process, so this is the process whose `open / limit` ratio actually matters.

In the UI each container shows a `Files: <open>/<limit>` line — colour-coded
green / orange / red as it approaches the limit — and an open-files **history
graph** next to the CPU/memory graphs: a steadily climbing line is a
file-descriptor leak. The values are also in the `files` block of
`GET /containers`.

The container's **Processes** button opens a dialog listing every process
inside the container with its own open file descriptors and `nofile` limit
(busiest process first) — useful for pinning down which process leaks. It is
backed by `GET /containers/processes` and computed on demand from
`docker top` + the host `/proc`.

File descriptors are a *per-process* kernel resource — the Docker Engine API
does not expose them. The collector obtains them by reading the host `/proc`:
for each container it gets the main process PID from `docker inspect`
(`/containers/{id}/json` → `State.Pid`), then counts `<proc>/<pid>/fd` and reads
the `Max open files` line of `<proc>/<pid>/limits`.

### Paths to mount into the collector container

Because the collector runs inside Docker, the host `/proc` must be made visible
to it. **Mount it as a volume** — this is the single extra path required, on top
of the Docker socket:

| Host path | Mount inside the container | Mode        | Why                                              |
| --------- | -------------------------- | ----------- | ------------------------------------------------ |
| `/proc`   | `/host/proc`               | `ro` (read) | Per-container open file descriptors and `nofile` limits |

```yaml
volumes:
- /var/run/docker.sock:/var/run/docker.sock
- /proc:/host/proc:ro
- ./.docker-statistics-collector:/root/.docker-statistics-collector:ro
```

The mount point is configurable via the `host_proc_path` setting (default
`/host/proc`). The bind-mounted `/proc` carries the host's process table, so the
host PIDs returned by Docker resolve correctly. The collector must run as `root`
(the default) to read `<proc>/<pid>/fd` of processes owned by other users.

**Alternative — `pid: host`.** Instead of the volume mount you can give the
collector the host PID namespace; then its own `/proc` already shows host
processes and you set `host_proc_path: /proc`:

```yaml
pid: host
```

### When the stats are unavailable

`files.open` / `files.limit` stay `null` (UI shows `N/A`) when the host `/proc`
cannot be read — i.e. the volume is not mounted, the Docker daemon is on a
**remote** host (`docker_url` points elsewhere), or the platform has no `/proc`
(e.g. running the collector on macOS for development). This is non-fatal: CPU and
memory stats keep working regardless.

## Host disk statistics

The collector reports the **host machine's physical disks** — which filesystems
exist and how much space is used on each. This is **host-level** information (the
physical machine the collector runs on), not per-container: it rides alongside
host memory/CPU in the `disks` array of each entry in the `hosts` list of
`GET /api/containers`, tagged with the same `instance` name and federated across
peers.

Each disk entry has `device` (e.g. `/dev/sda1`), `mountPoint` (e.g. `/`),
`fsType` (e.g. `ext4`), and `total` / `used` / `available` in bytes.

Only filesystems backed by a real block device (`/dev/...`) are reported;
pseudo-filesystems (tmpfs, overlay, proc, sysfs, cgroup, …) are filtered out, and
each physical device is listed once. To hide specific disks (e.g. the EFI
partition), list them under `ignore_disks` — each entry matches either the
device (`/dev/sda15`) or the mount point (`/boot/efi`).

### Why a second mount is required

Free/used space is **not** exposed anywhere in `/proc` — it comes from the
`statvfs(2)` syscall on each mount point, which needs the actual host filesystem
visible inside the container. The collector:

1. enumerates the host mount table from `<host_proc_path>/mounts` (the existing
   `/proc` mount), then
2. calls `statvfs` on `<host_root_path><mountPoint>` to measure the **host**
   filesystem rather than the collector container's own view.

So one extra volume is required, on top of the Docker socket and `/proc`:

| Host path | Mount inside the container | Mode        | Why                                          |
| --------- | -------------------------- | ----------- | -------------------------------------------- |
| `/`       | `/host/root`               | `ro` (read) | `statvfs` each host filesystem for disk usage |

```yaml
volumes:
- /var/run/docker.sock:/var/run/docker.sock
- /proc:/host/proc:ro
- /:/host/root:ro
- ./.docker-statistics-collector:/root/.docker-statistics-collector:ro
```

The mount is **read-only** and recursive (Docker bind-mounts are recursive by
default), so filesystems mounted on separate disks (e.g. `/data` on another
drive) are measured too. The target path is configurable via the
`host_root_path` setting (default `/host/root`).

### When the stats are unavailable

The `disks` array is empty when `/host/root` is not mounted (nothing to
`statvfs`), the Docker daemon is on a **remote** host, or the platform has no
`/proc` to enumerate mounts (e.g. macOS for development). Non-fatal — every other
stat keeps working.

## Federation

When `peers` is set in the master collector's settings, every request that needs
cross-host data **fans out to peers in real time** (parallel HTTP calls,
`peers_request_timeout_secs` per call, default 5s). There is no peer cache and
no sync timer — adding/removing peers in the YAML is picked up on the next
request.

- `GET /api/containers` and `GET /api/containers/running` return the union of
  local containers and a real-time fan-out to every peer's
  `/api/containers/local`. Each item carries an `instance` field naming the
  source's `ENV_INFO`. Failed peers are logged to stderr and skipped
  (best-effort merge).
- `GET /api/containers/logs?id=...&lines_number=...` tries the local Docker
  socket first; if the id is unknown locally it broadcasts the log fetch to all
  peers in parallel and returns the first 200 response.
- `GET /api/containers/local` is the **peer-facing** endpoint — local data only,
  never re-fanouts. This makes A↔B reciprocal peering safe (no recursion).
- The `/mcp` tools behave the same way — searches and log retrieval span the
  fleet through the master.
- `/metrics` aggregation across peers is **not** federated in this version.

## MCP endpoint

The collector exposes an MCP (Model Context Protocol) endpoint at `POST /mcp`
on the same port (`8000`). Three tools are exposed: `list_servers_and_services`,
`find_containers`, `get_container_logs`. All federation-aware.

See [MCP.md](MCP.md) for the full guide — endpoint, tool schemas, response
shapes, client registration, sample tool-call flows, and notes on extending the
server.

Quick client config:

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
