# docker-statistics-hub

Backend REST + WebSocket service for the `docker-statistics-ui` client-side WASM frontend.

## What it does

- Reads its `envs` config from `~/.docker-statistics-hub`
- Polls each env's master `docker-statistics-collector` every 3 seconds via `GET /api/containers`
- Keeps a per-env in-memory cache of container metrics + history. **The collector stores
  nothing** — it answers every request from a live Docker scan — so this service is the
  only side that remembers anything, and a restart starts from an empty cache.
- Keeps on disk only what cannot be read back from a collector: the names operators
  give to host disks, and the history of what the polls measured — see
  [Service metrics](#service-metrics). All of it lives in one
  [data folder](#data-folder), a file or a folder per kind of data.
- **Derives network throughput.** The collector ships raw cumulative `rx_bytes`/`tx_bytes`
  plus the instant they were read; `NetSample::rate_to` turns two consecutive readings
  into the `in_mbps`/`out_mbps` the UI shows. This requires a collector built after the
  stateless refactor — pointed at an older one, the Network column stays blank.
- **Does NOT measure disk sizes.** The collector runs its own background timer for
  those and ships the result in every payload; this service just stores what arrives.
- Enforces per-user access to envs via the `x-ssl-user` header set by the upstream reverse proxy
- Exposes endpoints consumed by the WASM UI:
  - `GET  /api/envs` — list envs visible to the current user
  - `GET  /api/vm_cpu_and_mem?env&selected_vm` — VM aggregates + optional per-container details
  - `GET  /api/logs?env&url&id&lines_amount` — one-shot proxy of container logs from the env's master collector
  - `GET  /api/processes?env&url&id` — one-shot proxy of container processes
  - `POST /api/disk-title` — name a host disk (JSON body: `env`, `vm`, `disk`, `title`); see [Disk titles](#disk-titles)
  - `GET  /api/service-metrics?env&vm&service&metric&from&to` — history of one metric of one service, up to 48 hours per request; see [Service metrics](#service-metrics)
  - `WS   /ws/logs?env&id&tail=N` — live log stream proxied from the collector's `/ws/logs?id` endpoint

Listens on `0.0.0.0:8000`.

## Formerly docker-statistics-api

Up to `0.2.20` this service was called `docker-statistics-api`. The rename moved
everything that carried the name:

|               | before                                     | now                                        |
| ------------- | ------------------------------------------ | ------------------------------------------ |
| release tag   | `docker-statistics-api-<version>`          | `docker-statistics-hub-<version>`          |
| image         | `ghcr.io/myjettools/docker-statistics-api` | `ghcr.io/myjettools/docker-statistics-hub` |
| settings file | `~/.docker-statistics-api`                 | `~/.docker-statistics-hub`                 |
| data folder   | `~/.docker-statistics-api-data`            | `~/.docker-statistics-hub-data`            |

Upgrading a deployment means retargeting its two mounts to the new paths inside
the container. The host side can stay where it is — the same settings file and
the same data folder — so disk titles saved before the rename are kept.

## Settings

`~/.docker-statistics-hub` (YAML):

```yaml
envs:
  prod:
    url: http://collector-master-prod:8080
  staging:
    url: http://collector-master-staging:8080
  dev:
    url: http://collector-master-dev:8080

# ── RBAC (optional) ─────────────────────────────────────────────────────────
# If `users` is omitted entirely → no RBAC, every caller sees every env (dev).
# If `users` is present:
#   - the caller's identity is taken from the `x-ssl-user` request header
#     (set by the upstream reverse proxy; we never validate it ourselves)
#   - a user not listed in `users` sees no envs at all
#   - a user mapped to the special group `*` sees all envs
#   - otherwise the user sees the intersection of `user_groups[their group]`
#     with the configured `envs`
# Access is enforced uniformly on REST endpoints AND the WS log stream — a
# user that cannot see the env can neither read its metrics nor subscribe to
# its logs.

users:
  amigin@gmail.com: admins        # admins group
  contractor@vendor.com: dev-only # gets the "dev-only" group below
  ceo@example.com: "*"           # sees every env

user_groups:
  admins: [prod, staging, dev]
  dev-only: [dev]

# ── Service metrics (optional) ───────────────────────────────────────────────
# How many days of metric history are kept on disk; older day files are deleted.
# Default 7 — some 2 GB for a hundred services. See "Service metrics" below.
metrics_days_to_keep: 7
```

## Data folder

`~/.docker-statistics-hub-data` is the one folder this service writes to. Mount
it as a volume (see [Deployment](#deployment-docker-compose)) and everything the
service keeps survives the container; nothing else needs mounting for that.

Each kind of data is a **file of its own** in the folder — or a folder of its
own, when it is many files. Keeping something new later means one more entry
here — the mount stays as it is.

| Entry              | What it holds                                                                           |
| ------------------ | --------------------------------------------------------------------------------------- |
| `disk-titles.yaml` | Names given to host disks in the UI — see [Disk titles](#disk-titles)                   |
| `metrics/`         | What the polls measured, service by service, in files by the day — see [Service metrics](#service-metrics) |

**Storing something new** — the rule for whoever changes this service next: do
not add a mount, a second top-level folder, or a section to a file that is about
something else. Give the data a name of its own and take it from `DataFolder`
([data_folder.rs](src/app/data_folder.rs)): `file(name)` the way `DiskTitles`
([disk_titles.rs](src/app/disk_titles.rs)) does, or `folder(name)` for a kind of
data that is many files, the way `MetricsStorage`
([metrics_storage.rs](src/metrics_storage/metrics_storage.rs)) does. Create the
store next to them in `AppCtx::new`, and add a row to the table above.

The folder is created on the first write. A file taken with `file(name)` is
replaced atomically — written beside itself and renamed — which is why the folder
is mounted and not a single file: a rename onto a file that is itself a mount
point is refused. Without the mount everything lives in the container's own
filesystem: it is gone when the container is recreated, and until then the
metrics grow there, by gigabytes.

## Disk titles

A host disk can be given a name in the UI — click its icon in the VM rail. The
title is shown in place of the mount point, there and on the Host disks board.
Everything else this service holds is a cache of what the collectors report; a
title somebody typed is not, so it is kept in the [data folder](#data-folder).

`disk-titles.yaml`:

```yaml
# env -> vm -> mount point -> title
prod:
  vm-prod-db-01:
    /var/lib/postgresql: Postgres data
```

- A disk is identified by its **mount point**, not its device: `/dev/sdX` can
  move when a volume is re-attached, the mount point does not.
- `POST /api/disk-title` sets a title; an empty (or absent) `title` removes it.
  Titles are trimmed, single-line and at most 48 characters. The caller needs
  access to the env, same as for every other endpoint.
- The file is rewritten on every change and read once, at startup — after
  editing it by hand, restart the service. A file that cannot be parsed is left
  alone: titles stay off and saving is refused until it is fixed or removed.

## Service metrics

Every poll is also written down: what each service measured goes to disk, so its
history outlives both the in-memory cache and a restart. The cache keeps the last
minutes for the live graphs; this keeps days.

| Metric    | Value                         | Limit kept next to it           |
| --------- | ----------------------------- | ------------------------------- |
| `cpu`     | cpu in use, as the collector reports it | cores of the host     |
| `mem`     | bytes of memory in use        | memory limit of the container   |
| `files`   | open file descriptors         | its `nofile` limit              |
| `net-in`  | inbound throughput, MB/s      | —                               |
| `net-out` | outbound throughput, MB/s     | —                               |

- A **service** is a container by its name, not its id: the id changes with every
  release, the name does not, so the history runs on through releases. Services
  are kept per env and per VM.
- A measurement is stamped with the second the collector took it. A reading that
  measured nothing — a stopped container, stats that did not read — leaves no
  record.
- The files are `metrics/<env>/<vm>/<YYYYMMDD>.<metric>` in the
  [data folder](#data-folder): one per VM, UTC day and metric, in a binary format
  of this service's own (described in [frame.rs](src/metrics_storage/frame.rs)).
  A record is 20 bytes, so a service polled every 3 seconds takes about 3 MB a day
  across its five metrics.
- Days older than `metrics_days_to_keep` (default `7`) are deleted, checked every
  10 minutes. The default is some 2 GB for a hundred services.

`GET /api/service-metrics?env&vm&service&metric&from&to` reads it back: `from`
(inclusive) and `to` (exclusive) are unix seconds, at most 48 hours apart;
`service` is the container name as the UI shows it; `metric` is one of the five
above. The caller needs access to the env. The answer is three columns of one
length, in time order — an interval nothing was measured in is three empty ones:

```json
{ "time": [1791763200, 1791763203], "value": [0.5, 1.5], "limit": [8.0, null] }
```

`limit` is `null` where the value has none.

## Deployment (docker-compose)

Standard compose template that runs the API alongside the static UI host on
one machine. The reverse-proxy sitting in front of `docker-statistics-ui` is
expected to forward `/api/*` and `/ws/*` to `docker-statistics-hub:8000` and
to inject the `x-ssl-user` header on authenticated requests.

```yaml
services:
  docker-statistics-ui:
    image: ghcr.io/myjettools/docker-statistics-ui:0.2.12
    hostname: docker-statistics-ui
    container_name: docker-statistics-ui
    restart: always
    environment:
    - ENV_INFO
    ports:
    - "8011:8000"
    deploy:
      resources:
        limits:
           memory: 128Mb
    logging:
      options:
        max-size: "512Kb"
        max-file: "1"
    networks:
    - docker_net

  docker-statistics-hub:
    image: ghcr.io/myjettools/docker-statistics-hub:0.2.12
    hostname: docker-statistics-hub
    container_name: docker-statistics-hub
    restart: always
    environment:
    - ENV_INFO
    volumes:
    - /var/run/docker.sock:/var/run/docker.sock
    - ./.docker-statistics-hub:/root/.docker-statistics-hub:ro
    - ./docker-statistics-hub-data:/root/.docker-statistics-hub-data
    deploy:
      resources:
        limits:
           memory: 128Mb
    logging:
      options:
        max-size: "512Kb"
        max-file: "1"
    networks:
    - docker_net

networks:
  docker_net:
    external: true
```

Notes on the mounts:
- `./.docker-statistics-hub` — your settings YAML (see [Settings](#settings)).
- `./docker-statistics-hub-data` — the [data folder](#data-folder): the one
  writable mount, holding a file per kind of data the service keeps. Mount the
  folder, not the files in it.
- `/var/run/docker.sock` — only needed if this same host also runs a local
  collector that the api talks to over the same socket; otherwise drop it.
- `~/unix-sockets/*` — shared unix-socket dirs used when api talks to other
  on-host services over uds; drop those that don't apply to your setup.

## WebSocket: live logs

`WS /ws/logs?env=<env>&id=<container_id>&tail=N`

- `env` — required, must be one configured in `envs`
- `id` — required, full container id
- `tail` — optional initial backfill of N lines (default 200)

The api opens an upstream WS to `ws://<master>/ws/logs?id&tail` on the env's
master collector, forwards every text/binary frame to the browser, and sends
a Ping every 5 seconds to keep the connection alive during quiet periods.
Closing the browser tab drops both legs cleanly.

Each text frame is one log line as JSON: `{"tp": <stream>, "line": "<text>"}`,
where `tp=1` is stdout, `tp=2` is stderr (docker's multiplexed framing).
