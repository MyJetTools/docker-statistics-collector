# Collector

`docker-statistics-collector` exposes the Docker Engine API of one host over HTTP:
container information, runtime statistics (CPU, memory, network, file descriptors)
and the Prometheus metrics of containers running under `docker compose`. One runs
on every Docker host; how to run it is in the [README](../README.md#1-collector--on-every-docker-host).

It is built on [docker-sdk/](../docker-sdk/) — a minimal async client for the Docker
Engine HTTP API.

## How it works

**The collector stores nothing.** It runs no background timers and keeps no
container cache — every endpoint reads the Docker host live and answers with
what it found during that request. Polling, aggregation, history and the
derived values that need more than one sample are the job of
`docker-statistics-hub`, which is the service that remembers anything.

That split shows up in three places:

1. **Container statistics** — each request lists containers and, for every
   running one, fetches runtime stats plus a combined inspect (start time and
   file-descriptor usage) in parallel. See
   [live_containers.rs](../docker-statistics-collector/src/app/live_containers.rs).
   File-descriptor stats require the host `/proc` to be mounted — see
   [File descriptor statistics](file-descriptor-statistics.md).
   Concurrent callers (the hub's poll, a peer fan-out, an MCP tool) are
   **single-flighted**: one of them performs the scan and the rest wait for its
   result, so a burst of requests is still one pass over the daemon.
2. **Network throughput** — throughput needs two samples, and a stateless
   service has only one. The collector therefore ships the *raw* cumulative
   `rx_bytes`/`tx_bytes` counters together with `sampled_at_unix_ms`, and the
   API derives MB/s from consecutive readings.
3. **Prometheus metrics** — scraped on demand, never cached. For every container
   carrying the `com.docker.compose.service` label the collector fetches
   `http://<service_name>:<metrics_port>/metrics`; if the response looks like
   Prometheus text format an `app="<service_name>"` label is injected into every
   metric line. See
   [metrics_scraper.rs](../docker-statistics-collector/src/metrics_scraper.rs).
   The scrape stays on the collector for a physical reason: those URLs are
   compose service names, which only resolve from inside that host's Docker
   network — a central service cannot reach them.

**Disk usage is the one thing the collector remembers.** Docker walks the storage
layers to size a container — far too slow to do on the request path — so a background
timer measures one container per 5s tick, parks the answer in memory, and every
containers payload carries the latest known values. That timer is the collector's only
timer and that cache is its only state; the hub just reads the numbers along
with everything else on its own polling schedule, and measures nothing itself. See
[sync_disk_sizes_timer.rs](../docker-statistics-collector/src/timers/sync_disk_sizes_timer.rs).

The HTTP server listens on port `8000` and exposes a Swagger UI. See
[start_up.rs](../docker-statistics-collector/src/http/start_up.rs).

## HTTP endpoints

- `GET /api/containers` — list of containers with their state, image, ports, volumes
  and a live usage snapshot (CPU, memory, raw network counters, and a `files`
  block with open file descriptors and the `nofile` limit), plus the `disk` block
  filled by the background size timer. Federated: includes every peer's containers.
- `GET /api/containers/running` — only running containers.
- `GET /api/containers/local` — this instance's containers only; the endpoint peers
  call each other on.
- `GET /api/containers/logs?id=&lines_number=` — container logs.
- `GET /api/containers/processes?id=` — every process inside one container with its
  open file descriptors and `nofile` limit (federation-aware, like logs).
- `POST /api/containers/exec` — run a command inside a container. Gated by the
  exec-permission unlock below.
- `GET /api/exec-permission`, `POST /api/exec-permission/enable`,
  `POST /api/exec-permission/disable` — the time-limited unlock for `exec`.
- `GET /metrics` — Prometheus scrape target. Scraped from every compose service at
  request time, with a 2s per-service budget so one dead exporter cannot blank the
  whole page.
- `GET /api/metrics/list` — the compose services that answered with Prometheus
  content on this call.
- `GET /api/metrics?id=` — metrics for a single service, scraped on demand.
- `ws://…/ws/logs` and `ws://…/ws/exec` — the streaming counterparts of logs and exec.

Controller sources: [containers_controller/](../docker-statistics-collector/src/http/controllers/containers_controller/),
[metrics_controller/](../docker-statistics-collector/src/http/controllers/metrics_controller/).

## Settings

Settings are read from `~/.docker-statistics-collector` (YAML). See
[settings.rs](../docker-statistics-collector/src/settings.rs).

| Field                        | Type           | Description                                                                       |
| ---------------------------- | -------------- | --------------------------------------------------------------------------------- |
| `docker_url`                 | `string`       | Docker Engine API endpoint. TCP: `http://localhost:2375`. Unix socket: `http+unix://var/run/docker.sock`. |
| `metrics_port`               | `u16`          | Port on which each service exposes its Prometheus `/metrics` endpoint.            |
| `disable_metics_collecting`  | `bool?`        | If `true`, the metrics endpoints answer empty without scraping anything.           |
| `services_to_ignore`         | `list<string>?`| Optional. `com.docker.compose.service` values to skip during scraping.            |
| `peers`                      | `list<string>?`| Optional. Base URLs of peer collector instances to federate with. `http://` and `https://` both work, and a peer can be pinned to a known IP with `https://name@ip:port` — see [Peer addresses](federation.md#peer-addresses). |
| `peers_request_timeout_secs` | `u64?`         | Optional. Per-peer request timeout. Default `30`. **Must exceed a peer's worst-case full live Docker scan** — a peer that misses it drops out of the fleet view for that tick. |
| `host_proc_path`             | `string?`      | Optional. Path inside the collector container where the host `/proc` is mounted. Used to read per-container open file descriptors and `nofile` limits. Default `/host/proc`. See [File descriptor statistics](file-descriptor-statistics.md). |
| `host_root_path`             | `string?`      | Optional. Path inside the collector container where the host root filesystem is bind-mounted (`-v /:/host/root:ro`). Used to `statvfs` each host mount point for physical-disk usage. Default `/host/root`. See [Host disk statistics](host-disk-statistics.md). |
| `ignore_disks`               | `list<string>?`| Optional. Physical disks to hide from host disk stats. Each entry matches either the block device (`/dev/sda15`) or the mount point (`/boot/efi`). |
| `exec_unlock_duration_secs`  | `u64?`         | Optional. How long one "enable exec" click keeps the `exec_in_container` MCP tool unlocked on this instance. Default `600` (10 minutes). See [MCP endpoint](mcp.md). |

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
  - https://collector-c.example.com:8443
  # Known IP, no DNS: connect to 10.0.0.7, but send Host / TLS SNI as collector-d.
  - https://collector-d.example.com@10.0.0.7:8443
peers_request_timeout_secs: 30
# Path where the host `/proc` is mounted inside the collector container.
# Default is `/host/proc` — see file-descriptor-statistics.md for the
# required `-v /proc:/host/proc:ro` volume mount.
host_proc_path: /host/proc
# Path where the host root filesystem is bind-mounted inside the collector
# container. Default is `/host/root` — see host-disk-statistics.md for the
# required `-v /:/host/root:ro` volume mount.
host_root_path: /host/root
# Disks to hide from host disk stats — matched by device or mount point.
ignore_disks:
  - /boot/efi
  - /dev/sdf
```

The HTTP listen port (`8000`) is hardcoded in
[start_up.rs](../docker-statistics-collector/src/http/start_up.rs).

### `ENV_INFO`

The `ENV_INFO` environment variable is **required** and must be non-empty: the
collector refuses to start without it. It is the name this host goes by — it tags
every container in federated responses and is what the UI shows as the VM — so it
**must be unique across peered collectors**. See
[main.rs](../docker-statistics-collector/src/main.rs).

## Building it

```bash
cd docker-statistics-collector
cargo build --release
./target/release/docker-statistics-collector
```

Needs Rust (edition 2021) and access to a Docker Engine API endpoint. Scraped
services must expose Prometheus text-format metrics on `/metrics` at the
configured `metrics_port`. The image is that binary on `ubuntu:22.04` — see the
[Dockerfile](../docker-statistics-collector/Dockerfile).

## Related

- [File descriptor statistics](file-descriptor-statistics.md)
- [Host disk statistics](host-disk-statistics.md)
- [Federation](federation.md)
- [MCP endpoint](mcp.md)
