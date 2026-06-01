/// Host physical-disk usage, read from the host's mount table + `statvfs`.
///
/// This is **host-level** information (the physical machine the collector runs
/// on), not per-container. One [`DiskSnapshot`] per mounted physical filesystem.
///
/// Two things are needed from the host, because free/used space is NOT exposed
/// anywhere in `/proc` — it comes from the `statvfs(2)` syscall on each mount
/// point:
///   * `proc_base`  — the host `/proc` (already bind-mounted at `/host/proc`),
///                    used only to enumerate the mount table (`/proc/mounts`).
///   * `root_base`  — the host root filesystem, recursively bind-mounted
///                    read-only (recommended `-v /:/host/root:ro`). `statvfs`
///                    is called on `{root_base}{mount_point}` so it measures the
///                    HOST filesystem, not the collector container's own view.
///
/// If `root_base` is not mounted, every `statvfs` fails and the result is empty
/// — that is the signal the operator forgot the `-v /:/host/root:ro` volume.
#[derive(Clone, Debug)]
pub struct DiskSnapshot {
    /// Block device, e.g. `/dev/sda1`, `/dev/nvme0n1p2`.
    pub device: String,
    /// Mount point on the host, e.g. `/`, `/data`.
    pub mount_point: String,
    /// Filesystem type, e.g. `ext4`, `xfs`, `btrfs`.
    pub fs_type: String,
    /// Total size in bytes.
    pub total: i64,
    /// Used bytes (`total` minus free).
    pub used: i64,
    /// Bytes available to unprivileged users.
    pub available: i64,
}

/// Read all physical-disk filesystems and their usage.
///
/// Returns an empty vec when the mount table can't be read (no host `/proc`) or
/// when no physical filesystem could be measured (host root not mounted).
pub fn read(proc_base: &str, root_base: &str, ignore: &[String]) -> Vec<DiskSnapshot> {
    let mounts = match read_host_mount_table(proc_base) {
        Some(content) => content,
        None => return Vec::new(),
    };

    let mut result = Vec::new();
    let mut seen_devices = std::collections::HashSet::new();

    for (device, mount_point, fs_type) in parse_physical_mounts(&mounts) {
        // Same physical device can appear several times (bind mounts, btrfs
        // subvolumes) — report each disk once.
        if !seen_devices.insert(device.clone()) {
            continue;
        }

        // Operator-configured hide list — match by device or mount point.
        if is_ignored(&device, &mount_point, ignore) {
            continue;
        }

        let probe_path = join_under_root(root_base, &mount_point);
        if let Some((total, available, free)) = statvfs_bytes(&probe_path) {
            result.push(DiskSnapshot {
                device,
                mount_point,
                fs_type,
                total,
                used: (total - free).max(0),
                available,
            });
        }
    }

    result
}

/// A disk is hidden when the operator's `ignore_disks` list contains its block
/// device (e.g. `/dev/sda15`) or its mount point (e.g. `/boot/efi`).
fn is_ignored(device: &str, mount_point: &str, ignore: &[String]) -> bool {
    ignore.iter().any(|i| i == device || i == mount_point)
}

/// Read the HOST's mount table.
///
/// `<proc_base>/mounts` is a symlink to `self/mounts`, which resolves to the
/// COLLECTOR process — and that process lives in the container's mount
/// namespace, so it lists overlay/bind mounts, not the host's physical disks.
/// The host's real mount table lives in PID 1's namespace, so we read
/// `<proc_base>/1/mounts` (host init). We fall back to `self/mounts` only if
/// PID 1 is unreadable, so a misconfigured deployment still returns *something*.
fn read_host_mount_table(proc_base: &str) -> Option<String> {
    std::fs::read_to_string(format!("{}/1/mounts", proc_base))
        .or_else(|_| std::fs::read_to_string(format!("{}/mounts", proc_base)))
        .ok()
}

/// Parse `/proc/mounts`, keeping only filesystems backed by a real block device
/// (`/dev/...`). This drops the pseudo-filesystems (tmpfs, proc, sysfs, cgroup,
/// overlay, devtmpfs, …) that aren't "physical disks".
fn parse_physical_mounts(content: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let Some(device) = parts.next() else { continue };
        let Some(mount_point) = parts.next() else {
            continue;
        };
        let Some(fs_type) = parts.next() else { continue };

        if !device.starts_with("/dev/") {
            continue;
        }
        if is_pseudo_fs(fs_type) {
            continue;
        }

        out.push((
            unescape_mount(device),
            unescape_mount(mount_point),
            fs_type.to_string(),
        ));
    }
    out
}

/// Safety net on top of the `/dev/` device filter — some virtual filesystems
/// (devtmpfs, squashfs loop images) sit on `/dev/...` paths but aren't durable
/// physical storage we care about for capacity.
fn is_pseudo_fs(fs_type: &str) -> bool {
    matches!(
        fs_type,
        "tmpfs"
            | "devtmpfs"
            | "proc"
            | "sysfs"
            | "cgroup"
            | "cgroup2"
            | "overlay"
            | "squashfs"
            | "mqueue"
            | "devpts"
            | "debugfs"
            | "tracefs"
            | "autofs"
            | "ramfs"
    )
}

/// `/proc/mounts` octal-escapes space (`\040`), tab (`\011`), newline (`\012`)
/// and backslash (`\134`). Decode those back.
fn unescape_mount(raw: &str) -> String {
    if !raw.contains('\\') {
        return raw.to_string();
    }
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            let octal = &raw[i + 1..i + 4];
            if let Ok(code) = u8::from_str_radix(octal, 8) {
                out.push(code as char);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Join a host mount point under the bind-mounted host root, avoiding a double
/// slash. `("/host/root", "/")` -> `/host/root`; `("/host/root", "/data")` ->
/// `/host/root/data`.
fn join_under_root(root_base: &str, mount_point: &str) -> String {
    let root = root_base.trim_end_matches('/');
    if mount_point == "/" {
        if root.is_empty() {
            "/".to_string()
        } else {
            root.to_string()
        }
    } else {
        format!("{}{}", root, mount_point)
    }
}

/// Call `statvfs` and return `(total, available, free)` in bytes, or `None` when
/// the path can't be measured (not mounted / doesn't exist).
fn statvfs_bytes(path: &str) -> Option<(i64, i64, i64)> {
    let c_path = std::ffi::CString::new(path).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }

    // `f_frsize` is the fundamental block size; fall back to `f_bsize`.
    let unit = if stat.f_frsize > 0 {
        stat.f_frsize as i64
    } else {
        stat.f_bsize as i64
    };

    let total = stat.f_blocks as i64 * unit;
    let free = stat.f_bfree as i64 * unit;
    let available = stat.f_bavail as i64 * unit;

    if total <= 0 {
        return None;
    }
    Some((total, available, free))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_physical_device_mounts() {
        let sample = "\
sysfs /sys sysfs rw,nosuid 0 0
proc /proc proc rw,nosuid 0 0
/dev/sda1 / ext4 rw,relatime 0 0
tmpfs /run tmpfs rw,nosuid 0 0
/dev/nvme0n1p1 /boot vfat rw,relatime 0 0
overlay /var/lib/docker/overlay2/abc/merged overlay rw 0 0
/dev/sda1 /var/snap ext4 rw,relatime 0 0
";
        let mounts = parse_physical_mounts(sample);
        let devices: Vec<&str> = mounts.iter().map(|(d, _, _)| d.as_str()).collect();
        // sysfs/proc/tmpfs/overlay dropped; both /dev/sda1 lines kept by parser
        // (dedup happens in read()).
        assert_eq!(devices, vec!["/dev/sda1", "/dev/nvme0n1p1", "/dev/sda1"]);
        assert_eq!(mounts[0].1, "/");
        assert_eq!(mounts[0].2, "ext4");
        assert_eq!(mounts[1].1, "/boot");
        assert_eq!(mounts[1].2, "vfat");
    }

    #[test]
    fn decodes_octal_escapes_in_mount_point() {
        // "/mnt/my disk" with the space octal-escaped as \040.
        assert_eq!(unescape_mount("/mnt/my\\040disk"), "/mnt/my disk");
        assert_eq!(unescape_mount("/data"), "/data");
    }

    #[test]
    fn ignore_matches_device_or_mount_point() {
        let ignore = vec!["/boot/efi".to_string(), "/dev/sdf".to_string()];
        assert!(is_ignored("/dev/sda15", "/boot/efi", &ignore)); // by mount point
        assert!(is_ignored("/dev/sdf", "/mnt/vol", &ignore)); // by device
        assert!(!is_ignored("/dev/sda1", "/", &ignore)); // kept
        assert!(!is_ignored("/dev/sda1", "/", &[])); // empty list keeps everything
    }

    #[test]
    fn joins_mount_point_under_root_without_double_slash() {
        assert_eq!(join_under_root("/host/root", "/"), "/host/root");
        assert_eq!(join_under_root("/host/root", "/data"), "/host/root/data");
        assert_eq!(join_under_root("/host/root/", "/data"), "/host/root/data");
        assert_eq!(join_under_root("", "/"), "/");
    }
}
