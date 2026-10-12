# docker-statistics

Monitoring of Docker containers across many hosts: a **collector** on every host,
one **hub** that polls them and keeps what has to be kept, and a web **ui**.

```
browser ──► reverse proxy ─┬─ /api/  /ws/  ──► docker-statistics-hub ──► master collector ──► peer collectors
                           └─ everything else ► docker-statistics-ui      (one per env)        (one per host)
```

This file says **how to run the services**. How each part works inside is in
[docs/](docs/README.md), one file per component — the list is
[at the end](#components).

## What to run

| Service   | Where                 | Image                                            | Settings file                    |
| --------- | --------------------- | ------------------------------------------------ | -------------------------------- |
| collector | on every Docker host  | `ghcr.io/myjettools/docker-statistics-collector` | `~/.docker-statistics-collector` |
| hub       | once per installation | `ghcr.io/myjettools/docker-statistics-hub`       | `~/.docker-statistics-hub`       |
| ui        | next to the hub       | `ghcr.io/myjettools/docker-statistics-ui`        | none                             |

All three listen on port `8000` inside their container; the port is not
configurable. Start them in this order: the collectors, the hub, the ui, then the
[proxy routes](#4-reverse-proxy).

**Which version.** An image tag is the version of a release tag
`docker-statistics-<service>-<version>`. The examples below say `<version>`; the
latest one of each service is:

```bash
git tag -l 'docker-statistics-collector-*' --sort=-v:refname | head -1
git tag -l 'docker-statistics-hub-*'       --sort=-v:refname | head -1
git tag -l 'docker-statistics-ui-*'        --sort=-v:refname | head -1
```

The hub was called `docker-statistics-api` up to `0.2.20` — see
[Moving from docker-statistics-api](#moving-from-docker-statistics-api).

## 1. Collector — on every Docker host

**Settings** — `~/.docker-statistics-collector` (YAML). Two fields are required:

```yaml
docker_url: http+unix://var/run/docker.sock   # or http://localhost:2375
metrics_port: 9091                            # where compose services expose /metrics
```

One collector of an environment is the **master** — the one the hub talks to. It
is an ordinary collector whose settings also list the others; the others need to
know nothing about it or about each other:

```yaml
peers:
  - http://collector-b:8000
  - https://collector-c.example.com:8443
```

Every other field is optional — see [Collector → Settings](docs/collector.md#settings).

**`ENV_INFO`** — a required environment variable: the name this host goes by in the
UI. It must be unique among the collectors of an environment, and the collector
refuses to start without it.

**Mounts:**

| Host path                        | Inside the container             | Mode | What stops working without it          |
| -------------------------------- | -------------------------------- | ---- | -------------------------------------- |
| `/var/run/docker.sock`           | `/var/run/docker.sock`           | any  | everything — unless `docker_url` is a TCP address; then leave this mount out |
| `/proc`                          | `/host/proc`                     | ro   | open files per container               |
| `/`                              | `/host/root`                     | ro   | disks of the host                      |
| `~/.docker-statistics-collector` | `/root/.docker-statistics-collector` | ro | the collector does not start        |

```yaml
services:
  docker-statistics-collector:
    image: ghcr.io/myjettools/docker-statistics-collector:<version>
    hostname: docker-statistics-collector
    container_name: docker-statistics-collector
    restart: always
    environment:
    - ENV_INFO=vm-prod-01
    volumes:
    - /var/run/docker.sock:/var/run/docker.sock
    - /proc:/host/proc:ro
    - /:/host/root:ro
    - ~/.docker-statistics-collector:/root/.docker-statistics-collector:ro
    ports:
    - "8000:8000"
    deploy:
      resources:
        limits:
          memory: 128Mb
```

The published port is what goes into the master's `peers` and into the hub's
`envs` — any free host port will do.

**Check:** `curl -s http://localhost:8000/api/containers/local` lists the
containers of this host, each tagged with the `ENV_INFO` name.

## 2. Hub

**Settings** — `~/.docker-statistics-hub` (YAML). One entry per environment,
pointing at its master collector as the hub's container reaches it:

```yaml
envs:
  prod:
    url: http://collector-master-prod:8000
  staging:
    url: http://collector-master-staging:8000
```

Optional: who may see which environment — [Access control](docs/access-control.md);
how many days of history are kept — [Service metrics](docs/service-metrics.md).
All fields: [Hub → Settings](docs/hub.md#settings).

**Mounts:**

| Host path                 | Inside the container              | Mode | What it is |
| ------------------------- | --------------------------------- | ---- | ---------- |
| the settings file         | `/root/.docker-statistics-hub`      | ro | the hub has no environment to poll without it |
| a folder for the hub's data | `/root/.docker-statistics-hub-data` | rw | everything the hub keeps: disk titles, metric history |

Mount the data **folder**, not files in it. Without the mount the hub still works,
but what it keeps is lost when the container is recreated, and until then the
metric history grows inside the container. Allow about 3 MB per container per
day; a week is kept by default.

## 3. UI

Static files and nothing else: no settings, no mounts, no environment. It calls
`/api/` and `/ws/` on the address it was opened at, which is why it needs the
[reverse proxy](#4-reverse-proxy) to put the hub there.

The hub and the ui in one compose file:

```yaml
services:
  docker-statistics-hub:
    image: ghcr.io/myjettools/docker-statistics-hub:<version>
    hostname: docker-statistics-hub
    container_name: docker-statistics-hub
    restart: always
    volumes:
    - ~/.docker-statistics-hub:/root/.docker-statistics-hub:ro
    - ~/docker-statistics-hub-data:/root/.docker-statistics-hub-data
    deploy:
      resources:
        limits:
          memory: 128Mb
    networks:
    - docker_net

  docker-statistics-ui:
    image: ghcr.io/myjettools/docker-statistics-ui:<version>
    hostname: docker-statistics-ui
    container_name: docker-statistics-ui
    restart: always
    deploy:
      resources:
        limits:
          memory: 128Mb
    networks:
    - docker_net

networks:
  docker_net:
    external: true
```

Neither publishes a port: the reverse proxy reaches them by name on the shared
network.

## 4. Reverse proxy

The browser has to find the ui and the hub at **one address**:

| Path              | Goes to                             |
| ----------------- | ----------------------------------- |
| `/api/`           | `http://docker-statistics-hub:8000` |
| `/ws/`            | `http://docker-statistics-hub:8000` — WebSocket |
| everything else   | `http://docker-statistics-ui:8000`  |

The proxy is also what authenticates the user: it puts the identity into the
`x-ssl-user` header, and the hub trusts that header as it comes. So the hub must
not be reachable past the proxy. With no `users` in the hub's settings every
caller sees every environment — see [Access control](docs/access-control.md).

**Check:** `https://<your address>/api/envs` answers
`{"envs":["prod","staging"],"user_id":"…"}`, and the page lists the hosts a few
seconds after the hub starts. If it does not, `docker logs docker-statistics-hub`
says which master collector it failed to reach.

## Moving from docker-statistics-api

Releases up to `0.2.20` were `docker-statistics-api`. Everything that carried the
name moved with it, so a deployment of the old service is changed in four places:

| What          | Before                                     | Now                                        |
| ------------- | ------------------------------------------ | ------------------------------------------ |
| image         | `ghcr.io/myjettools/docker-statistics-api` | `ghcr.io/myjettools/docker-statistics-hub` |
| container     | `docker-statistics-api`                    | `docker-statistics-hub`                    |
| settings file | `/root/.docker-statistics-api`             | `/root/.docker-statistics-hub`             |
| data folder   | `/root/.docker-statistics-api-data`        | `/root/.docker-statistics-hub-data`        |

1. Replace the service in the compose file with the one [above](#3-ui).
2. Keep the host side of both mounts as it is — the same settings file, the same
   data folder — and change only the paths inside the container. Disk titles
   saved before the move stay.
3. Point the proxy's `/api/` and `/ws/` at `docker-statistics-hub:8000`.
4. Remove the old container.

## Components

How each part works, in detail — [docs/](docs/README.md):

### Collector

- [Collector](docs/collector.md)
- [File descriptor statistics](docs/file-descriptor-statistics.md)
- [Host disk statistics](docs/host-disk-statistics.md)
- [Federation](docs/federation.md)
- [MCP endpoint](docs/mcp.md)

### Hub

- [Hub](docs/hub.md)
- [Access control](docs/access-control.md)
- [Data folder](docs/data-folder.md)
- [Disk titles](docs/disk-titles.md)
- [Service metrics](docs/service-metrics.md)
- [Live logs](docs/live-logs.md)

### UI

- [UI](docs/ui.md)

### Development

- [Building and releasing](docs/releasing.md)
