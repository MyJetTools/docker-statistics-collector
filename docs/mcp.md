# MCP endpoint

The collector exposes an MCP (Model Context Protocol) endpoint at `POST /mcp`, on
the same port as its HTTP API (`8000`). Every tool is federation-aware: asked of
the master collector, it answers for the whole fleet, and each result carries an
`instance` field naming the host a container runs on.

Client config:

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

## Tools

| Tool | What it does |
| --- | --- |
| `list_servers_and_services` | Instances and the compose services on each, with their port mappings |
| `find_application` | Regex match on the identity of an application: compose service, container names, image |
| `find_containers` | Substring search across id, names, image and labels, with a CPU / memory snapshot |
| `get_container_logs` | Tail of a container's logs, routed to the host that owns it |
| `get_compose_yaml` | The docker-compose.yaml a container was started from, read from its `com.release-mcp.compose-yaml` label |
| `get_host_info` | RAM, CPU count and physical disks of every host |
| `list_exposed_ports` | Every host-published port per instance |
| `exec_in_container` | Runs a shell command inside a container. **Locked by default** — see below |

They are registered in [start_up.rs](../docker-statistics-collector/src/http/start_up.rs)
and implemented in [mcp/](../docker-statistics-collector/src/mcp/).

What a model is told about them is not written here but in two files compiled into
the binary, which is where to edit it:

- [MCP_INSTRUCTION.md](../docker-statistics-collector/MCP_INSTRUCTION.md) — the
  server instructions every session gets on `initialize`;
- [MCP_PROMPT_HOW_TO_USE_IT.md](../docker-statistics-collector/MCP_PROMPT_HOW_TO_USE_IT.md)
  — the `how_to_use_it` prompt, loaded on demand.

### The exec unlock

`exec_in_container` does nothing until a person unlocks it: in the UI, on the
panel of a container of that VM, "enable exec" opens a window of
`exec_unlock_duration_secs` (10 minutes by default) on that VM only, and it closes
by itself. The window is enforced by the collector that owns the container, so
unlocking one VM never opens exec anywhere else. The same window is behind
`GET /api/exec-permission` and `POST /api/exec-permission/enable|disable`.

The rest of this file is the usage guide of the three tools most sessions start
with.

## Recommended flow

1. **Discover topology** with `list_servers_and_services` — get the list of instances and which compose services run on each (with port mappings).
2. **Drill into a specific service or keyword** with `find_containers` — get full per-container detail (id, image, ports, CPU/memory, labels).
3. **Read logs** with `get_container_logs` — pass any `id` returned by `find_containers`. Routing across hosts is automatic; no `instance` parameter is needed.

## `list_servers_and_services`

Returns a list of instances and the compose services running on each, with deduplicated port mappings per service.

**Input**

| Field | Type | Default | Meaning |
|---|---|---|---|
| `only_running` | `bool` | `true` | If `true`, only running containers contribute. |

**Output**

```json
{
  "servers": [
    {
      "instance": "vm-prod-01",
      "container_count": 5,
      "services": [
        {
          "service_name": "metrics-api",
          "ports": [
            { "host_ip": "10.0.0.5", "host_port": 4555, "container_port": 9090, "protocol": "tcp" }
          ]
        }
      ]
    }
  ]
}
```

**Reading port mappings**

- `host_ip == "" && host_port == null` → port is **not published** to the host (only reachable inside the container network).
- `host_ip == "0.0.0.0"` → published on all host interfaces.
- `host_ip == "10.0.0.5"` → published only on that specific host IP.
- `container_port` is the port inside the container that `host_port` maps to.

## `find_containers`

Substring search (case-insensitive) across container `id`, `names`, `image`, and label keys/values, on every instance the server can see.

**Input**

| Field | Type | Default | Meaning |
|---|---|---|---|
| `phrase` | `string` | required | Non-empty substring to look for. |
| `only_running` | `bool` | `true` | If `true`, only running containers are returned. |

**Output**

```json
{
  "containers": [
    {
      "id": "9a4c...e1b",
      "instance": "vm-prod-02",
      "names": ["/redis-cache"],
      "image": "redis:7.2",
      "state": "running",
      "status": "Up 4 hours",
      "running": true,
      "compose_service": "redis",
      "cpu_usage": 0.012,
      "mem_usage": 18653184,
      "mem_limit": 268435456,
      "labels": [
        { "label_key": "com.docker.compose.service", "label_value": "redis" }
      ],
      "ports": [
        { "host_ip": "0.0.0.0", "host_port": 6379, "container_port": 6379, "protocol": "tcp" }
      ]
    }
  ]
}
```

**Field semantics**

- `instance` — the host the container runs on. Distinguishes results when the same image runs on multiple hosts.
- `compose_service` — value of the `com.docker.compose.service` label, or `""` if absent.
- `cpu_usage` is fractional CPU cores (e.g. `0.5` = half a core, `2.0` = two cores).
- `mem_usage`, `mem_limit` are in bytes.
- `labels` is a list of `{ label_key, label_value }` entries (not a map).
- `ports` follow the same semantics as in `list_servers_and_services`.

## `get_container_logs`

Fetches the tail of a container's combined stdout/stderr.

**Input**

| Field | Type | Default | Meaning |
|---|---|---|---|
| `container_id` | `string` | required | Container id (full or prefix) returned by `find_containers`. |
| `tail` | `u32` | `200` | Number of trailing lines to return. |

**Output**

```json
{
  "logs": "2025-05-03T08:00:00Z INFO booting...\n2025-05-03T08:00:01Z INFO listening on :9090\n..."
}
```

The id is enough — the server resolves which host owns the container automatically. Returns an error if the id is not recognized on any reachable host.

## Examples

> *"Which servers run a `metrics-api` and on what ports?"*
> → `list_servers_and_services` → filter `services[].service_name == "metrics-api"`, read `ports[].host_ip:host_port → container_port`.

> *"Find every nginx container and show me the last 100 log lines from the one on `vm-prod-02`."*
> → `find_containers(phrase: "nginx")` → pick the entry with `instance == "vm-prod-02"` → `get_container_logs(container_id, tail: 100)`.

> *"Is the redis on prod-02 reachable from outside the host?"*
> → `find_containers(phrase: "redis")` → for the entry with `instance == "vm-prod-02"`, inspect `ports[*].host_ip` / `host_port` (empty = not exposed; `0.0.0.0` = exposed on all interfaces).
