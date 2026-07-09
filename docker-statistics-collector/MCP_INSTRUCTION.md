# MCP server instructions

Sent to MCP clients as `ServerInfo.instructions` on `initialize`. Loaded at
compile time via `include_str!` from `src/http/start_up.rs`. Edit this file to
change what the model sees as system context for every session.

---

This server gives access to Docker containers across one or more hosts via
seven tools (`list_servers_and_services`, `find_containers`,
`get_container_logs`, `get_compose_yaml`, `get_host_info`, `exec_in_container`,
`list_exposed_ports`) plus prompts that carry deeper guidance.

`list_exposed_ports` returns every host-published port per instance (sorted),
so you can see which ports are taken on each VM and choose the next free one.

`get_compose_yaml` returns the decoded docker-compose.yaml that produced a
container, read from its `com.release-mcp.compose-yaml` label (gzip+base64) and
auto-routed to the owning instance or peer. Errors if the container has no such
label.

`exec_in_container` runs a shell command inside a container (like `docker exec`,
as `sh -c "<command>"`) and returns its stdout/stderr and exit code, auto-routed
to the owning instance or peer.

**`exec_in_container` is dangerous and is DISABLED by default.** Two rules govern it:

1. **It must be unlocked by a human.** A person opens the Docker Statistics UI for
   that VM and presses "Enable exec", which unlocks the tool on that VM only, for
   10 minutes (`exec_unlock_duration_secs`), after which it locks itself again.
   The unlock is enforced on the VM that owns the container, so unlocking one VM
   never opens exec anywhere else. If the window is closed the tool returns an
   error — do not retry it, tell the user to enable it first.
2. **You must announce the commands before running them.** Before your first call,
   write out in plain text the exact command(s) you intend to execute and what each
   is for, and wait for the user to approve. Never execute a command the user has
   not seen. Prefer read-only commands (`ls`, `cat`, `ps`, `env`); never run
   destructive ones (`rm`, `kill`, writes, package installs) unless the user asked
   for that exact command.

The command runs under **POSIX `sh`, not bash** (busybox `ash` on Alpine images),
so bashisms such as `[[ ]]`, arrays and `pipefail` will not work.

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
