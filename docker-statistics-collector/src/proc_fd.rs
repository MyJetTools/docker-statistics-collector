use docker_sdk::container_inspect::get_container_main_pid;
use docker_sdk::container_top::get_container_processes;

/// Collects file-descriptor usage for a container's **main process** — the
/// process started by the image `ENTRYPOINT`/`CMD` (PID 1 inside the
/// container). `RLIMIT_NOFILE` is enforced per process, so this is the process
/// whose `open / limit` ratio matters and whose growth over time reveals a
/// file-descriptor leak.
///
/// Returns `(open, limit)`: file descriptors currently open, and the `nofile`
/// soft limit. Both are `None` when the data cannot be obtained — the host
/// `/proc` is not mounted into the collector container, the Docker daemon is
/// remote, or the platform has no `/proc` (macOS). Callers treat that as "N/A".
pub async fn collect_fd_usage(
    docker_url: &str,
    proc_base: &str,
    container_id: &str,
) -> (Option<i64>, Option<i64>) {
    let pid = match get_container_main_pid(docker_url.to_string(), container_id.to_string()).await {
        Some(pid) => pid,
        None => return (None, None),
    };

    let proc_base = proc_base.to_string();

    // /proc reads are blocking std::fs calls — keep them off the async runtime.
    match tokio::task::spawn_blocking(move || read_fd_usage(&proc_base, pid)).await {
        Ok(value) => value,
        Err(_) => (None, None),
    }
}

fn read_fd_usage(proc_base: &str, pid: u32) -> (Option<i64>, Option<i64>) {
    let open = count_open_fds(proc_base, pid).map(|count| count as i64);
    let limit = read_nofile_soft_limit(proc_base, pid);
    (open, limit)
}

/// File-descriptor usage of a single process inside a container.
pub struct ProcessFdInfo {
    pub pid: u32,
    pub cmd: String,
    pub open_files: Option<i64>,
    pub fd_limit: Option<i64>,
}

/// Lists every process inside a container with its open file descriptors and
/// `nofile` soft limit — the data behind the per-process "Processes" dialog.
/// Returns an empty list when the process list or the host `/proc` is
/// unavailable.
pub async fn collect_process_fd_list(
    docker_url: &str,
    proc_base: &str,
    container_id: &str,
) -> Vec<ProcessFdInfo> {
    let processes =
        match get_container_processes(docker_url.to_string(), container_id.to_string()).await {
            Some(processes) => processes,
            None => return Vec::new(),
        };

    let proc_base = proc_base.to_string();

    // /proc reads are blocking std::fs calls — keep them off the async runtime.
    let result = tokio::task::spawn_blocking(move || {
        processes
            .into_iter()
            .map(|process| ProcessFdInfo {
                pid: process.pid,
                cmd: process.cmd,
                open_files: count_open_fds(&proc_base, process.pid).map(|count| count as i64),
                fd_limit: read_nofile_soft_limit(&proc_base, process.pid),
            })
            .collect()
    })
    .await;

    result.unwrap_or_default()
}

/// Counts the entries in `<proc_base>/<pid>/fd` — the file descriptors the
/// process currently has open. Only the directory is listed (no symlink
/// resolution), so `CAP_DAC_OVERRIDE` — part of Docker's default capability
/// set — is enough; `CAP_SYS_PTRACE` is not required. `None` when the directory
/// cannot be read (process gone, no `/proc`, or insufficient permissions).
fn count_open_fds(proc_base: &str, pid: u32) -> Option<usize> {
    let entries = std::fs::read_dir(format!("{}/{}/fd", proc_base, pid)).ok()?;
    Some(entries.filter(|entry| entry.is_ok()).count())
}

/// Reads the `nofile` soft limit (`Max open files`) from
/// `<proc_base>/<pid>/limits`. `None` when the file is unavailable or the limit
/// is `unlimited`.
fn read_nofile_soft_limit(proc_base: &str, pid: u32) -> Option<i64> {
    let content = std::fs::read_to_string(format!("{}/{}/limits", proc_base, pid)).ok()?;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("Max open files") {
            let soft_limit = rest.split_whitespace().next()?;
            return soft_limit.parse::<i64>().ok();
        }
    }

    None
}
