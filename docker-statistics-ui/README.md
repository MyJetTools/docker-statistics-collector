# docker-statistics-ui

Web UI for monitoring Docker containers across multiple machines and environments. Built on [Dioxus](https://dioxuslabs.com/) (fullstack + web) in Rust.

The UI talks to **one** federated [`docker-statistics-collector`](../docker-statistics-collector/) per environment. That collector is the "master": it runs on one of the hosts in the env and is configured with `peers` pointing at every other collector in the same env. When the UI asks for data, the master fans out to its peers in real time and returns a merged view. The UI does not call peers directly.

## Features

- One master URL per environment (`prod`, `staging`, ...) — no per-VM URL list.
- Each container in the response is tagged with its source instance (`ENV_INFO` of the collector that physically runs it). The sidebar groups by that tag, so the user sees containers split by machine even though only one HTTP endpoint is hit.
- Per-VM and aggregated ("All VMs") CPU / memory / container counts.
- Per-container live CPU and memory history graphs (150-point sliding window, refreshed each second).
- Container filter by id / image / name / label.
- Show / hide disabled containers.
- Port, label, state, status, and "created" age visualization.
- Logs viewer dialog per container — the master auto-routes the log fetch to whichever peer owns the container.
- Optional per-user environment access control.
- Optional interactive SSH pass-phrase prompt on startup (when reaching the master through an SSH tunnel).

## Architecture

```
                ┌──────────────────────┐
   browser ───► │   docker-statistics  │      ← settings: envs.<name>.url
                │           ui         │        (one master per env)
                └──────────┬───────────┘
                           │  GET /api/containers      (every 3s, per env)
                           ▼
              ┌────────────────────────┐
              │   master collector     │      ← settings.peers: [...]
              │   (any host in env)    │
              └─┬──────────────────────┘
                │ fan out in real time at request time
   ┌────────────┼────────────┬────────────┐
   ▼            ▼            ▼            ▼
 host A       host B       host C       host D
 collector    collector    collector    collector
   ↑ each only knows about its OWN docker socket
```

- The master is just a regular `docker-statistics-collector` — its only difference is the `peers` field in its YAML.
- Logs are routed transparently: UI hits `/api/containers/logs?id=...` on the master; the master proxies to the right peer.
- No proactive peer sync, no shared cache. Adding/removing peers in the master's YAML is picked up on the next request.

## Settings

Settings are loaded from `~/.docker-statistics-ui` (YAML).

Top-level keys: `envs`, `ssh_private_keys`, `prompt_pass_phrase`, `users`, `user_groups`.

### Plain HTTP(S) endpoints

Both `http://` and `https://` schemes are accepted.

```yaml
envs:
  prod:
    url: http://10.0.0.2:7999       # the master collector for prod
  staging:
    url: http://10.0.1.2:7999       # the master collector for staging
```

The master at `10.0.0.2:7999` itself has, in its `~/.docker-statistics-collector`:

```yaml
docker_url: http+unix://var/run/docker.sock
metrics_port: 9091
peers:
  - http://10.0.0.3:7999
  - http://10.0.0.4:7999
```

The peer collectors (`10.0.0.3`, `10.0.0.4`) need only their own `docker_url`; they do **not** need to know about each other or about the master.

### SSH tunneling

The url scheme `ssh:user@host:port->http://target:port` opens an SSH tunnel to the master. With one URL per env, the SSH config collapses to one tunnel per env:

```yaml
envs:
  prod:
    url: ssh:gateway@10.0.0.0:22->http://10.0.0.2:7999
  staging:
    url: ssh:gateway@10.0.0.1:22->http://10.0.1.2:7999

ssh_private_keys:
  "gateway@10.0.0.0:22":
    cert_path: /root/cert-1
    cert_pass_prase: password
  "gateway@10.0.0.1:22":
    cert_path: /root/cert-2
    cert_pass_prase: password
```

A single shared key is also supported by using the `"*"` wildcard:

```yaml
ssh_private_keys:
  "*":
    cert_path: /root/cert
    cert_pass_prase: password
```

`ssh_private_keys` can be omitted entirely — in that case the running SSH agent is used.

### Prompting for SSH pass-phrase at startup

If you prefer not to store the private-key pass-phrase in the settings file, set `prompt_pass_phrase: true`. On first request the UI asks for the pass-phrase and holds it in memory for the lifetime of the process.

```yaml
prompt_pass_phrase: true

ssh_private_keys:
  "*":
    cert_path: /root/cert
```

### Per-user environment access control

When the server sits behind a reverse proxy that injects an `x-ssl-user` header, environments can be gated by user. Each user maps to a group; groups list the environments they may see. The special group `"*"` grants access to every environment.

```yaml
envs:
  prod:
    url: http://10.0.0.2:7999
  staging:
    url: http://10.0.1.2:7999

users:
  alice@example.com: admins
  bob@example.com:   developers

user_groups:
  admins:
    - prod
    - staging
  developers:
    - staging
```

If `users` is not defined, all environments are visible to everyone. When `users` is defined, any request whose `x-ssl-user` value is not listed (or whose group is missing from `user_groups`) sees an empty list of environments. Assigning the value `"*"` directly to a user (e.g. `alice@example.com: "*"`) bypasses `user_groups` and grants access to every environment.

## Migration from the old multi-URL format

Earlier versions accepted a list of URLs per env:

```yaml
# Old — no longer supported
envs:
  prod:
    - url: http://10.0.0.2:7999
    - url: http://10.0.0.3:7999
    - url: http://10.0.0.4:7999
```

Replace it with a single master URL and configure the federation on the collector side (the master's `peers` field):

```yaml
# New
envs:
  prod:
    url: http://10.0.0.2:7999
```

## Running

### Locally (development)

Requires the [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/) (`dx`).

```bash
dx serve --platform web
```

Defaults: `IP=0.0.0.0`, `PORT=9001` (inside Docker). Override via env vars.

### Docker

The container image is based on `ghcr.io/myjettools/dioxus-docker:0.7.7` and listens on port `9001` (`IP=0.0.0.0`, `PORT=9001`). The Dockerfile expects the release bundle to already be built on the host, so build the web assets first:

```bash
dx bundle --platform web --release
docker build -t docker-statistics-ui .
docker run -p 9001:9001 -v ~/.docker-statistics-ui:/root/.docker-statistics-ui docker-statistics-ui
```

### Cache-busting static assets

`build.py` rewrites references to `.wasm`, `.js`, and `.css` in a given HTML file to append a random `?id=...` query string. Run it against the generated `index.html` if you need to invalidate browser caches after a release:

```bash
python3 build.py target/dx/docker-statistics-ui/release/web/public/index.html
```

## Releasing

CI tags the UI image when a tag of the form `docker-statistics-ui-<version>` is pushed (see [`.github/workflows/release-docker-statistics-ui.yaml`](../.github/workflows/release-docker-statistics-ui.yaml)). Example:

```bash
git tag docker-statistics-ui-0.3.0
git push origin docker-statistics-ui-0.3.0
```

The workflow builds with `dx bundle`, uploads the bundle as an artifact, then builds and pushes `ghcr.io/myjettools/docker-statistics-ui:<version>`.
