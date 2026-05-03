# MCP server instructions

Sent to MCP clients as `ServerInfo.instructions` on `initialize`. Loaded at
compile time via `include_str!` from `src/http/start_up.rs`. Edit this file to
change what the model sees as system context for every session.

---

Inspect Docker containers across one or more hosts.

## Tools and recommended flow

1. `list_servers_and_services` — discover instances and the docker-compose
   services running on each, with their published port mappings.
2. `find_containers` — substring-search containers across all instances by
   id, name, image, or label. Returns CPU/memory snapshot, ports, labels, and
   `compose_service`.
3. `get_container_logs` — fetch tail logs by `container_id` (full or prefix
   returned by `find_containers`). Routing across hosts is automatic; do not
   pass an instance hint.

## Reading the `instance` field

Every result carries an `instance` field — the source host's identifier. Use
it to disambiguate when the same image runs on multiple hosts.

## Port mapping semantics

Both `list_servers_and_services` and `find_containers` use the same shape:

- `host_ip == ""` **and** `host_port == null` → not published; reachable only
  inside the container network.
- `host_ip == "0.0.0.0"` → published on every host interface.
- `host_ip == "<ip>"` → published only on that interface.
- `container_port` is the port inside the container that `host_port` maps to.

## Units and conventions

- `cpu_usage` is fractional CPU cores (`1.0` = one full core).
- `mem_usage` and `mem_limit` are bytes.
- `compose_service` is `""` when the container has no
  `com.docker.compose.service` label.
- `labels` is a list of `{ label_key, label_value }` entries (not a map).

## Deployment context

Containers reported by these tools may have been deployed in one of two ways:

1. **`docker compose`** — the typical case. Containers carry labels such as
   `com.docker.compose.service` and `com.docker.compose.project`, surfaced via
   `find_containers` `compose_service` and `labels`.
2. **`release-mcp`** ([github.com/my-ai-utils/release-mcp](https://github.com/my-ai-utils/release-mcp))
   — an internal release service that also lands containers on these hosts.
   Containers it deploys may not carry the `com.docker.compose.*` labels;
   absence of `compose_service` does **not** mean the container is orphaned or
   manually started.

When asked "where is X deployed", inspect both label sets in `find_containers`
output rather than relying solely on `compose_service`.

## Errors

If a tool returns an error mentioning a peer host, treat it as a transient
network problem affecting one host — other instances may still appear in the
response (best-effort merge).
