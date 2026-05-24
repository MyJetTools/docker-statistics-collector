# docker-statistics-api

Backend REST + WebSocket service for the `docker-statistics-ui` client-side WASM frontend.

## What it does

- Reads its `envs` config from `~/.docker-statistics-api`
- Polls each env's master `docker-statistics-collector` every 3 seconds via `GET /api/containers`
- Keeps a per-env in-memory cache of container metrics + history
- Enforces per-user access to envs via the `x-ssl-user` header set by the upstream reverse proxy
- Exposes endpoints consumed by the WASM UI:
  - `GET  /api/envs` — list envs visible to the current user
  - `GET  /api/vm_cpu_and_mem?env&selected_vm` — VM aggregates + optional per-container details
  - `GET  /api/logs?env&url&id&lines_amount` — one-shot proxy of container logs from the env's master collector
  - `GET  /api/processes?env&url&id` — one-shot proxy of container processes
  - `POST /api/pass_phrase` — submit SSH private-key passphrase (kept in process memory only)
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

# Optional: ask the UI for the SSH key passphrase on first connect
prompt_pass_phrase: false

# Optional: per-host SSH config for flurl SSH tunnels to a collector
ssh_private_keys:
  "*":
    cert_path: ~/.ssh/id_rsa

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
