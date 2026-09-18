use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::settings::SettingsModel;

use super::{DiskSizesCache, ServiceInfo};

/// What one scan publishes to everybody waiting on it — the snapshot, or the reason
/// there isn't one. Sharing the failure matters as much as sharing the success: without
/// it every follower becomes the next leader and repeats a scan that is already known
/// to be doomed.
type ScanResult = Result<Arc<Vec<ServiceInfo>>, Arc<String>>;

/// Live, on-demand reader of the local Docker host.
///
/// The collector deliberately keeps **no** container cache: every request walks the
/// Docker API and returns what it finds right now. The only state here is single-flight
/// bookkeeping — when several callers (the API poll timer, a peer fan-out and an MCP
/// tool, say) ask at the same moment, one of them performs the scan and the rest wait
/// for its result instead of piling more work onto the daemon.
/// Shared so the SCAN owns it, not the caller that happened to start it.
type Inflight = Arc<Mutex<Option<broadcast::Sender<ScanResult>>>>;

pub struct LiveContainers {
    settings_model: Arc<SettingsModel>,
    /// `Some` while a scan is in flight — everyone, leader included, subscribes to it.
    inflight: Inflight,
}

impl LiveContainers {
    pub fn new(settings_model: Arc<SettingsModel>) -> Self {
        Self {
            settings_model,
            inflight: Arc::new(Mutex::new(None)),
        }
    }

    /// Overlay the background timer's measurements onto a freshly scanned list. Kept
    /// outside the single-flighted scan so a snapshot served from an in-flight leader
    /// still carries sizes measured while it was running.
    async fn with_disk_sizes(
        disk_sizes: &DiskSizesCache,
        mut containers: Vec<ServiceInfo>,
    ) -> Vec<ServiceInfo> {
        let sizes = disk_sizes.get_snapshot().await;
        for container in containers.iter_mut() {
            if let Some(size) = sizes.get(&container.id) {
                container.size_rw = size.size_rw;
                container.size_root_fs = size.size_root_fs;
            }
        }
        containers
    }

    /// Full snapshot: container list plus, for every running container, its stats,
    /// file-descriptor usage and start time.
    pub async fn get_snapshot(
        &self,
        disk_sizes: &DiskSizesCache,
    ) -> Result<Vec<ServiceInfo>, String> {
        match self.get_shared_snapshot().await {
            Ok(snapshot) => Ok(Self::with_disk_sizes(disk_sizes, snapshot.as_ref().clone()).await),
            Err(err) => Err(err.as_ref().clone()),
        }
    }

    /// The container list ALONE — no per-container stats, no inspect. One daemon call
    /// instead of `1 + 2R`. Everything that only needs names, labels, ports or state
    /// belongs here rather than on [`get_snapshot`].
    pub async fn get_list(&self) -> Result<Vec<ServiceInfo>, String> {
        read_container_list(&self.settings_model).await
    }

    async fn get_shared_snapshot(&self) -> ScanResult {
        loop {
            let leader = {
                let mut inflight = self.inflight.lock().unwrap();
                match inflight.as_ref() {
                    Some(sender) => Err(sender.subscribe()),
                    None => {
                        let (sender, _) = broadcast::channel(1);
                        *inflight = Some(sender.clone());
                        Ok(sender)
                    }
                }
            };

            // Whether we started this scan or joined one, we wait the same way. The
            // scan itself lives in a detached task that owns the publishing, so a
            // caller going away (a peer fan-out that timed out, a closed tab) can
            // neither cancel the work the daemon has already paid for nor strand the
            // others waiting on it — which is exactly what happened while the guard
            // and the sender lived in the starter's future.
            let mut receiver = match leader {
                Ok(sender) => {
                    let receiver = sender.subscribe();
                    let settings_model = self.settings_model.clone();
                    let inflight = self.inflight.clone();

                    tokio::spawn(async move {
                        // Clears the slot even if the scan panics — otherwise `inflight`
                        // would keep a sender nobody will ever publish to, and every
                        // later caller would subscribe to it and wait forever.
                        let guard = InflightGuard { inflight };

                        let result: ScanResult = match read_snapshot(&settings_model).await {
                            Ok(snapshot) => Ok(Arc::new(snapshot)),
                            Err(err) => Err(Arc::new(err)),
                        };

                        // Cleared BEFORE publishing, so a caller arriving right after
                        // this starts a fresh scan rather than joining a finished one.
                        drop(guard);
                        // Ignore the error — it only means nobody was waiting.
                        let _ = sender.send(result);
                    });

                    receiver
                }
                Err(receiver) => receiver,
            };

            match receiver.recv().await {
                Ok(result) => return result,
                // The scan died before publishing — retry; this caller may well start
                // the next one.
                Err(_) => continue,
            }
        }
    }
}

struct InflightGuard {
    inflight: Inflight,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        *self.inflight.lock().unwrap() = None;
    }
}

/// Cheap variant: the plain container list, with no per-container stats calls.
pub async fn read_container_list(
    settings_model: &SettingsModel,
) -> Result<Vec<ServiceInfo>, String> {
    let sampled_at = now_unix_ms();
    let list = docker_sdk::list_of_containers::get_list_of_containers(
        settings_model.docker_url.to_string(),
    )
    .await?;

    Ok(list
        .iter()
        .map(|itm| ServiceInfo::from_container_json(itm, sampled_at))
        .collect())
}

async fn read_snapshot(settings_model: &SettingsModel) -> Result<Vec<ServiceInfo>, String> {
    let list_of_containers = docker_sdk::list_of_containers::get_list_of_containers(
        settings_model.docker_url.to_string(),
    )
    .await?;

    // The scan-start stamp is only a fallback for containers that produce no stats;
    // every container that does gets stamped at the moment its counters were actually
    // read (see below), because the api divides the counter delta by the delta of these
    // stamps and a scan that runs long would otherwise be charged to the rate.
    let scan_started_at = now_unix_ms();

    let mut result: Vec<ServiceInfo> = list_of_containers
        .iter()
        .map(|itm| ServiceInfo::from_container_json(itm, scan_started_at))
        .collect();

    let mut tasks = Vec::new();

    for container in list_of_containers {
        if !container.is_running() {
            continue;
        }

        let container_id = container.id.to_string();
        let url = settings_model.docker_url.to_string();
        let proc_base = settings_model.host_proc_path().to_string();

        tasks.push(tokio::spawn(async move {
            let usage =
                docker_sdk::sdk::get_container_stats(url.clone(), container_id.clone()).await;
            // Stamped HERE, next to the reading it describes — not once for the whole
            // scan. On a busy host the gap between the two is seconds, and it is the
            // *jitter* in that gap between polls that shows up as a saw-tooth on the
            // network chart.
            let sampled_at = now_unix_ms();

            // Combined inspect → started_at + FD usage in one daemon RTT.
            let probe = crate::proc_fd::probe_container(&url, &proc_base, &container_id).await;

            (container_id, usage, probe, sampled_at)
        }));
    }

    for task in tasks {
        let (container_id, usage, probe, sampled_at) = match task.await {
            Ok(value) => value,
            Err(err) => {
                // A panicking stats task costs this container one poll of data. Say so
                // rather than emitting an all-None row that reads as "container idle".
                eprintln!("container stats task panicked: {:?}", err);
                continue;
            }
        };

        let Some(item) = result.iter_mut().find(|itm| itm.id == container_id) else {
            continue;
        };

        if let Some(usage) = usage {
            item.cpu_usage = Some(usage.get_cpu_usage());
            item.mem_usage = Some(usage.get_used_memory());
            item.mem_available = Some(usage.get_available_memory());
            item.mem_limit = Some(usage.memory_stats.limit);
            item.net_rx_bytes = Some(usage.total_rx_bytes());
            item.net_tx_bytes = Some(usage.total_tx_bytes());
            item.net_sampled_at_unix_ms = sampled_at;
        }

        item.open_files = probe.open_files;
        item.fd_limit = probe.fd_limit;
        item.started_at = probe.started_at_unix_seconds;
    }

    Ok(result)
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
