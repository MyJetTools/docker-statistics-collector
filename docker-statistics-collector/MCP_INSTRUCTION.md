# MCP server instructions

Sent to MCP clients as `ServerInfo.instructions` on `initialize`. Loaded at
compile time via `include_str!` from `src/http/start_up.rs`. Edit this file to
change what the model sees as system context for every session.

---

This server gives access to Docker containers across one or more hosts via
four tools (`list_servers_and_services`, `find_containers`,
`get_container_logs`, `get_host_info`) plus prompts that carry deeper guidance.

`get_host_info` returns host-machine stats (NOT per-container) for this
collector and every peer: RAM, logical CPU count, and physical disks
(device, mount point, filesystem type, total/used/available bytes). Use it to
answer "how much disk space is left on each host". Disks are empty unless the
host root filesystem is bind-mounted into the collector (`-v /:/host/root:ro`).

## Prompts to load on demand

- `how_to_use_it` — load when the user asks to inspect Docker, look at
  containers, or look at the Docker console. Contains the full tool flow,
  field semantics, port mapping conventions, and deployment context.
- `release` — load when the user asks to start / make a release. Contains the
  release procedure for this project.
