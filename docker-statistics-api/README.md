# docker-statistics-api

Backend REST + WebSocket service for the `docker-statistics-ui` client-side WASM frontend.

## What it does

- Reads its `envs` config from `~/.docker-statistics-api`
- Polls each env's master `docker-statistics-collector` every 3 seconds via `GET /api/containers`
- Keeps a per-env in-memory cache of container metrics + history. **The collector stores
  nothing** — it answers every request from a live Docker scan — so this service is the
  only side that remembers anything, and a restart starts from an empty cache.
- Keeps on disk only what cannot be read back from a collector — today the names
  operators give to host disks. All of it lives in one [data folder](#data-folder),
  a file per kind of data.
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
  - `WS   /ws/logs?env&id&tail=N` — live log stream proxied from the collector's `/ws/logs?id` endpoint

Listens on `0.0.0.0:8000`.

## Settings

`~/.docker-statistics-api` (YAML):

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
```

## Data folder

`~/.docker-statistics-api-data` is the one folder this service writes to. Mount
it as a volume (see [Deployment](#deployment-docker-compose)) and everything the
service keeps survives the container; nothing else needs mounting for that.

Each kind of data is a **file of its own** in the folder. Keeping something new
later means one more file here — the mount stays as it is.

| File               | What it holds                                                        |
| ------------------ | -------------------------------------------------------------------- |
| `disk-titles.yaml` | Names given to host disks in the UI — see [Disk titles](#disk-titles) |

**Storing something new** — the rule for whoever changes this service next: do
not add a mount, a second folder, or a section to a file that is about something
else. Give the data a file name of its own and take that file from `DataFolder`
([data_folder.rs](src/app/data_folder.rs)), the way `DiskTitles`
([disk_titles.rs](src/app/disk_titles.rs)) does; create the store next to it in
`AppCtx::new`, and add a row to the table above.

The folder is created on the first write. A file is replaced atomically — written
beside itself and renamed — which is why the folder is mounted and not a single
file: a rename onto a file that is itself a mount point is refused. Without the
mount the files live in the container's own filesystem and are gone when the
container is recreated.

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

## Deployment (docker-compose)

Standard compose template that runs the API alongside the static UI host on
one machine. The reverse-proxy sitting in front of `docker-statistics-ui` is
expected to forward `/api/*` and `/ws/*` to `docker-statistics-api:8000` and
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

  docker-statistics-api:
    image: ghcr.io/myjettools/docker-statistics-api:0.2.12
    hostname: docker-statistics-api
    container_name: docker-statistics-api
    restart: always
    environment:
    - ENV_INFO
    volumes:
    - /var/run/docker.sock:/var/run/docker.sock
    - ./.docker-statistics-api:/root/.docker-statistics-api:ro
    - ./docker-statistics-api-data:/root/.docker-statistics-api-data
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
- `./.docker-statistics-api` — your settings YAML (see [Settings](#settings)).
- `./docker-statistics-api-data` — the [data folder](#data-folder): the one
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
