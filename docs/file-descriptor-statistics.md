# File descriptor statistics

For every running container the collector reports **how many file descriptors
its main process currently has open** and that process's **`nofile` soft
limit** (`RLIMIT_NOFILE`). The "main process" is the one started by the image
`ENTRYPOINT`/`CMD` — PID 1 inside the container. `RLIMIT_NOFILE` is enforced
per process, so this is the process whose `open / limit` ratio actually matters.

In the UI each container shows a `Files: <open>/<limit>` line — colour-coded
green / orange / red as it approaches the limit — and an open-files **history
graph** next to the CPU/memory graphs: a steadily climbing line is a
file-descriptor leak. The values are also in the `files` block of
`GET /api/containers`.

The container's **Processes** button opens a dialog listing every process
inside the container with its own open file descriptors and `nofile` limit
(busiest process first) — useful for pinning down which process leaks. It is
backed by `GET /api/containers/processes` and computed on demand from
`docker top` + the host `/proc`.

File descriptors are a *per-process* kernel resource — the Docker Engine API
does not expose them. The collector obtains them by reading the host `/proc`:
for each container it gets the main process PID from `docker inspect`
(`/containers/{id}/json` → `State.Pid`), then counts `<proc>/<pid>/fd` and reads
the `Max open files` line of `<proc>/<pid>/limits`.

## Paths to mount into the collector container

Because the collector runs inside Docker, the host `/proc` must be made visible
to it. **Mount it as a volume** — this is the single extra path required, on top
of the Docker socket:

| Host path | Mount inside the container | Mode        | Why                                              |
| --------- | -------------------------- | ----------- | ------------------------------------------------ |
| `/proc`   | `/host/proc`               | `ro` (read) | Per-container open file descriptors and `nofile` limits |

```yaml
volumes:
- /var/run/docker.sock:/var/run/docker.sock
- /proc:/host/proc:ro
- ./.docker-statistics-collector:/root/.docker-statistics-collector:ro
```

The mount point is configurable via the `host_proc_path` setting (default
`/host/proc`). The bind-mounted `/proc` carries the host's process table, so the
host PIDs returned by Docker resolve correctly. The collector must run as `root`
(the default) to read `<proc>/<pid>/fd` of processes owned by other users.

**Alternative — `pid: host`.** Instead of the volume mount you can give the
collector the host PID namespace; then its own `/proc` already shows host
processes and you set `host_proc_path: /proc`:

```yaml
pid: host
```

## When the stats are unavailable

`files.open` / `files.limit` stay `null` (UI shows `N/A`) when the host `/proc`
cannot be read — i.e. the volume is not mounted, the Docker daemon is on a
**remote** host (`docker_url` points elsewhere), or the platform has no `/proc`
(e.g. running the collector on macOS for development). This is non-fatal: CPU and
memory stats keep working regardless.
