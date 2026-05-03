# docker-statistics-ui

Web UI for monitoring Docker containers running across multiple VMs and environments. Built on [Dioxus](https://dioxuslabs.com/) (fullstack + web) in Rust.

The UI talks to remote `docker-statistics` agents over HTTP (direct or tunneled through SSH) and aggregates per-container CPU / memory / status / logs into a single dashboard.

## Features

- Multiple environments, each with one or more VMs.
- Direct HTTP or SSH-tunneled connections to remote agents.
- Per-VM and aggregated ("All VMs") CPU / memory / container counts.
- Per-container stats with live CPU and memory history graphs (150-point sliding window, refreshed each second).
- Container filter by id / image / name / label.
- Show / hide disabled containers.
- Port, label, state, status, and "created" age visualization.
- Logs viewer dialog per container with configurable line count.
- Optional per-user environment access control.
- Optional interactive SSH pass-phrase prompt on startup.

## Settings

Settings are loaded from `~/.docker-statistics-ui` (YAML).

Supported top-level keys: `envs`, `ssh_private_keys`, `prompt_pass_phrase`, `users`, `user_groups`.

### Plain HTTP(S) endpoints

Both `http://` and `https://` schemes are accepted.

```yaml
envs:
  env-1:
  - url: http://10.0.0.2:7999
  - url: http://10.0.0.3:7999
  - url: http://10.0.0.4:7999

  env-2:
  - url: http://10.0.1.2:7999
  - url: http://10.0.1.3:7999
  - url: http://10.0.1.4:7999
```

### SSH tunneling — single shared key

```yaml
envs:
  env-1:
  - url: ssh:gateway@10.0.0.0:22->http://10.0.0.2:7999
  - url: ssh:gateway@10.0.0.0:22->http://10.0.0.3:7999
  - url: ssh:gateway@10.0.0.0:22->http://10.0.0.4:7999

  env-2:
  - url: ssh:gateway@10.0.0.1:22->http://10.0.0.2:7999
  - url: ssh:gateway@10.0.0.1:22->http://10.0.0.3:7999
  - url: ssh:gateway@10.0.0.1:22->http://10.0.0.4:7999

ssh_private_keys:
  "*":
    cert_path: /root/cert
    cert_pass_prase: password
```

### SSH tunneling — key per gateway

```yaml
envs:
  env-1:
  - url: ssh:gateway@10.0.0.0:22->http://10.0.0.2:7999
  - url: ssh:gateway@10.0.0.0:22->http://10.0.0.3:7999
  - url: ssh:gateway@10.0.0.0:22->http://10.0.0.4:7999

  env-2:
  - url: ssh:gateway@10.0.0.1:22->http://10.0.0.2:7999
  - url: ssh:gateway@10.0.0.1:22->http://10.0.0.3:7999
  - url: ssh:gateway@10.0.0.1:22->http://10.0.0.4:7999

ssh_private_keys:
  "gateway@10.0.0.0:22":
    cert_path: /root/cert-1
    cert_pass_prase: password

  "gateway@10.0.0.1:22":
    cert_path: /root/cert-2
    cert_pass_prase: password
```

`ssh_private_keys` can be omitted — in that case the running SSH agent is used.

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
  - url: http://10.0.0.2:7999
  stage:
  - url: http://10.0.1.2:7999

users:
  alice@example.com: admins
  bob@example.com:   developers

user_groups:
  admins:
  - prod
  - stage
  developers:
  - stage
```

If `users` is not defined, all environments are visible to everyone. When `users` is defined, any request whose `x-ssl-user` value is not listed (or whose group is missing from `user_groups`) sees an empty list of environments. Assigning the value `"*"` directly to a user (e.g. `alice@example.com: "*"`) bypasses `user_groups` and grants access to every environment.

## Running

### Locally (development)

Requires the [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/) (`dx`).

```bash
dx serve --platform web
```

Defaults: `IP=0.0.0.0`, `PORT=9001` (inside Docker). Override via env vars.

### Docker

The container image is based on `ghcr.io/myjettools/dioxus-docker:0.7.5` and listens on port `9001` (`IP=0.0.0.0`, `PORT=9001`). The Dockerfile expects the release bundle to already be built on the host, so build the web assets first:

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
