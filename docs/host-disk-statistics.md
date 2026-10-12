# Host disk statistics

The collector reports the **host machine's physical disks** — which filesystems
exist and how much space is used on each. This is **host-level** information (the
physical machine the collector runs on), not per-container: it rides alongside
host memory/CPU in the `disks` array of each entry in the `hosts` list of
`GET /api/containers`, tagged with the same `instance` name and federated across
peers.

Each disk entry has `device` (e.g. `/dev/sda1`), `mountPoint` (e.g. `/`),
`fsType` (e.g. `ext4`), and `total` / `used` / `available` in bytes.

Only filesystems backed by a real block device (`/dev/...`) are reported;
pseudo-filesystems (tmpfs, overlay, proc, sysfs, cgroup, …) are filtered out, and
each physical device is listed once. To hide specific disks (e.g. the EFI
partition), list them under `ignore_disks` — each entry matches either the
device (`/dev/sda15`) or the mount point (`/boot/efi`).

## Why a second mount is required

Free/used space is **not** exposed anywhere in `/proc` — it comes from the
`statvfs(2)` syscall on each mount point, which needs the actual host filesystem
visible inside the container. The collector:

1. enumerates the host mount table from `<host_proc_path>/mounts` (the existing
   `/proc` mount), then
2. calls `statvfs` on `<host_root_path><mountPoint>` to measure the **host**
   filesystem rather than the collector container's own view.

So one extra volume is required, on top of the Docker socket and `/proc`:

| Host path | Mount inside the container | Mode        | Why                                          |
| --------- | -------------------------- | ----------- | -------------------------------------------- |
| `/`       | `/host/root`               | `ro` (read) | `statvfs` each host filesystem for disk usage |

```yaml
volumes:
- /var/run/docker.sock:/var/run/docker.sock
- /proc:/host/proc:ro
- /:/host/root:ro
- ./.docker-statistics-collector:/root/.docker-statistics-collector:ro
```

The mount is **read-only** and recursive (Docker bind-mounts are recursive by
default), so filesystems mounted on separate disks (e.g. `/data` on another
drive) are measured too. The target path is configurable via the
`host_root_path` setting (default `/host/root`).

## When the stats are unavailable

The `disks` array is empty when `/host/root` is not mounted (nothing to
`statvfs`), the Docker daemon is on a **remote** host, or the platform has no
`/proc` to enumerate mounts (e.g. macOS for development). Non-fatal — every other
stat keeps working.
