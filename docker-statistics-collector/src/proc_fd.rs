use docker_sdk::container_inspect::get_container_state;
use docker_sdk::container_top::get_container_processes;

/// Result of one inspect+proc cycle per container.
pub struct ContainerProbe {
    pub open_files: Option<i64>,
    pub fd_limit: Option<i64>,
    pub started_at_unix_seconds: Option<i64>,
}

/// Single inspect call per tick: returns started_at + FD usage. Saves a daemon
/// round trip vs. calling [`collect_fd_usage`] and [`get_container_state`]
/// separately during sync.
pub async fn probe_container(
    docker_url: &str,
    proc_base: &str,
    container_id: &str,
) -> ContainerProbe {
    let state = match get_container_state(docker_url.to_string(), container_id.to_string()).await {
        Some(s) => s,
        None => {
            return ContainerProbe {
                open_files: None,
                fd_limit: None,
                started_at_unix_seconds: None,
            };
        }
    };

    let (open, limit) = match state.pid {
        Some(pid) => {
            let proc_base = proc_base.to_string();
            tokio::task::spawn_blocking(move || read_fd_usage(&proc_base, pid))
                .await
                .unwrap_or((None, None))
        }
        None => (None, None),
    };

    ContainerProbe {
        open_files: open,
        fd_limit: limit,
        started_at_unix_seconds: state.started_at_unix_seconds,
    }
}

fn read_fd_usage(proc_base: &str, pid: u32) -> (Option<i64>, Option<i64>) {
    let open = count_open_fds(proc_base, pid).map(|count| count as i64);
    let limit = read_nofile_soft_limit(proc_base, pid);
    (open, limit)
}

/// File-descriptor usage and core stats of a single process inside a container.
pub struct ProcessFdInfo {
    pub pid: u32,
    pub cmd: String,
    pub open_files: Option<i64>,
    pub fd_limit: Option<i64>,
    /// Resident memory in bytes (`VmRSS`) — what's actually in RAM.
    pub mem_rss: Option<i64>,
    /// Virtual memory in bytes (`VmSize`) — full allocated address space.
    pub mem_vsize: Option<i64>,
    pub threads: Option<i64>,
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
            .map(|process| {
                let (mem_rss, mem_vsize, threads) = read_status_metrics(&proc_base, process.pid);
                ProcessFdInfo {
                    pid: process.pid,
                    cmd: process.cmd,
                    open_files: count_open_fds(&proc_base, process.pid).map(|count| count as i64),
                    fd_limit: read_nofile_soft_limit(&proc_base, process.pid),
                    mem_rss,
                    mem_vsize,
                    threads,
                }
            })
            .collect()
    })
    .await;

    result.unwrap_or_default()
}

/// Reads `VmRSS` (resident memory, bytes), `VmSize` (virtual memory, bytes)
/// and `Threads` (thread count) from `<proc_base>/<pid>/status` in a single
/// pass.
fn read_status_metrics(proc_base: &str, pid: u32) -> (Option<i64>, Option<i64>, Option<i64>) {
    let content = match std::fs::read_to_string(format!("{}/{}/status", proc_base, pid)) {
        Ok(content) => content,
        Err(_) => return (None, None, None),
    };

    let mut mem_rss_bytes: Option<i64> = None;
    let mut mem_vsize_bytes: Option<i64> = None;
    let mut threads: Option<i64> = None;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            if let Some(kb) = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<i64>().ok())
            {
                mem_rss_bytes = Some(kb * 1024);
            }
        } else if let Some(rest) = line.strip_prefix("VmSize:") {
            if let Some(kb) = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<i64>().ok())
            {
                mem_vsize_bytes = Some(kb * 1024);
            }
        } else if let Some(rest) = line.strip_prefix("Threads:") {
            if let Some(n) = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<i64>().ok())
            {
                threads = Some(n);
            }
        }
    }

    (mem_rss_bytes, mem_vsize_bytes, threads)
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
