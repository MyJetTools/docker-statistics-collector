# docker-statistics-api

Backend REST service for the `docker-statistics-ui` client-side WASM frontend.

## What it does

- Reads its `envs` config from `~/.docker-statistics-api`
- Polls each env's master `docker-statistics-collector` every 3 seconds via `GET /api/containers`
- Keeps a per-env in-memory cache of container metrics + history
- Exposes REST endpoints consumed by the WASM UI:
  - `GET  /api/envs` — list available envs
  - `GET  /api/vm_cpu_and_mem?env&selected_vm` — VM aggregates + optional per-container details
  - `GET  /api/logs?env&url&id&lines_amount` — proxy container logs through master collector
  - `GET  /api/processes?env&url&id` — proxy container processes through master collector
  - `POST /api/pass_phrase` — submit SSH private-key passphrase (in-memory only)

Listens on `0.0.0.0:9001`.

## Settings

`~/.docker-statistics-api` (YAML):

```yaml
envs:
  prod:
    url: http://collector-master-prod:8080
  staging:
    url: http://collector-master-staging:8080

prompt_pass_phrase: false   # optional: ask for SSH passphrase via UI

ssh_private_keys:           # optional: per-host SSH config for flurl --ssh tunnels
  "*":
    cert_path: ~/.ssh/id_rsa
```
