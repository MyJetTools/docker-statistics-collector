# Hub

`docker-statistics-hub` is the backend of the UI: it polls the collectors, keeps the
cache and the history, and is the only service of this repository that writes to
disk. One runs per installation; how to run it is in the
[README](../README.md#2-hub).

## What it does

- Reads its `envs` config from `~/.docker-statistics-hub`
- Polls each env's master `docker-statistics-collector` every 3 seconds via `GET /api/containers`
- Keeps a per-env in-memory cache of container metrics + history. **The collector stores
  nothing** — it answers every request from a live Docker scan — so this service is the
  only side that remembers anything, and a restart starts from an empty cache.
- Keeps on disk only what cannot be read back from a collector: the names operators
  give to host disks, and the history of what the polls measured — see
  [Service metrics](service-metrics.md). All of it lives in one
  [data folder](data-folder.md), a file or a folder per kind of data.
- **Derives network throughput.** The collector ships raw cumulative `rx_bytes`/`tx_bytes`
  plus the instant they were read; `NetSample::rate_to` turns two consecutive readings
  into the `in_mbps`/`out_mbps` the UI shows. This requires a collector built after the
  stateless refactor — pointed at an older one, the Network column stays blank.
- **Does NOT measure disk sizes.** The collector runs its own background timer for
  those and ships the result in every payload; this service just stores what arrives.
- Enforces per-user access to envs via the `x-ssl-user` header set by the upstream reverse
  proxy — see [Access control](access-control.md)
- Exposes endpoints consumed by the WASM UI:
  - `GET  /api/envs` — list envs visible to the current user
  - `GET  /api/vm_cpu_and_mem?env&selected_vm` — VM aggregates + optional per-container details
  - `GET  /api/logs?env&url&id&lines_amount` — one-shot proxy of container logs from the env's master collector
  - `GET  /api/processes?env&url&id` — one-shot proxy of container processes
  - `POST /api/disk-title` — name a host disk (JSON body: `env`, `vm`, `disk`, `title`); see [Disk titles](disk-titles.md)
  - `GET  /api/service-metrics?env&vm&service&metric&from&to` — history of one metric of one service, up to 48 hours per request; see [Service metrics](service-metrics.md)
  - `WS   /ws/logs?env&id&tail=N` — live log stream proxied from the collector's `/ws/logs?id` endpoint; see [Live logs](live-logs.md)

Listens on `0.0.0.0:8000`.

## Settings

`~/.docker-statistics-hub` (YAML). See
[settings_reader.rs](../docker-statistics-hub/src/settings_reader.rs).

| Field                  | Type                   | Description |
| ---------------------- | ---------------------- | ----------- |
| `envs`                 | `map<name, {url}>`     | The environments. One env is one federated master collector; `url` is its address — `http://`, `https://`, or `https://name@ip:port` to skip DNS (the forms of [peer addresses](federation.md#peer-addresses)). The master fans out to its own peers when it answers. |
| `users`                | `map<user, group>?`    | Optional. Who may see what — see [Access control](access-control.md). Without it everyone sees every env. |
| `user_groups`          | `map<group, [env]>?`   | Optional. The envs each group sees. |
| `metrics_days_to_keep` | `u32?`                 | Optional. Days of [service metrics](service-metrics.md) kept on disk. Default `7`. |

```yaml
envs:
  prod:
    url: http://collector-master-prod:8000
  staging:
    url: http://collector-master-staging:8000

users:
  alice@example.com: admins
  bob@example.com: "*"

user_groups:
  admins: [prod, staging]

metrics_days_to_keep: 7
```

The file is re-read while the service runs, so a change of `envs` or of the access
rules needs no restart. The listen port (`8000`) is hardcoded in
[start_up.rs](../docker-statistics-hub/src/http/start_up.rs), which also serves a
Swagger UI.

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

## Building it

```bash
cd docker-statistics-hub
cargo build --release
./target/release/docker-statistics-hub
```

The image is that binary on `ubuntu:22.04` — see the
[Dockerfile](../docker-statistics-hub/Dockerfile).

## Related

- [Access control](access-control.md)
- [Data folder](data-folder.md)
- [Disk titles](disk-titles.md)
- [Service metrics](service-metrics.md)
- [Live logs](live-logs.md)
