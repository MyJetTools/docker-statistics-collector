use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rust_extensions::{MyTimerTick, StopWatch};

use crate::app::AppContext;

/// Per-container disk usage is expensive (Docker walks the storage layers), so
/// we never compute the whole batch at once. Instead, every Nth *idle* tick we
/// refill `pending_disk` with all container ids, then drain it one container per
/// tick. Ticks that drain the queue do NOT count toward the next refill.
const DISK_SIZE_REFILL_EVERY_N_IDLE_TICKS: u64 = 10;

pub struct SyncContainersInfoTimer {
    app: Arc<AppContext>,
    /// Container ids still waiting for a disk-size pass (drained one per tick).
    pending_disk: Mutex<Vec<String>>,
    /// Idle ticks (queue empty) counted toward the next refill.
    idle_tick_no: AtomicU64,
}

impl SyncContainersInfoTimer {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self {
            app,
            pending_disk: Mutex::new(Vec::new()),
            idle_tick_no: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl MyTimerTick for SyncContainersInfoTimer {
    async fn tick(&self) {
        let sw = StopWatch::new();

        let list_of_containers = docker_sdk::list_of_containers::get_list_of_containers(
            self.app.settings_model.docker_url.to_string(),
        )
        .await;

        self.app.cache.update_services(&list_of_containers).await;

        // Disk usage — drain one container from the pending queue per tick. When
        // the queue is empty we count idle ticks and refill it (with all current
        // container ids) every Nth idle tick. Draining ticks don't count.
        let next_disk_id = {
            let mut pending = self.pending_disk.lock().unwrap();
            if pending.is_empty() {
                let n = self.idle_tick_no.fetch_add(1, Ordering::Relaxed) + 1;
                if n >= DISK_SIZE_REFILL_EVERY_N_IDLE_TICKS {
                    self.idle_tick_no.store(0, Ordering::Relaxed);
                    *pending = list_of_containers.iter().map(|c| c.id.clone()).collect();
                }
                pending.pop()
            } else {
                pending.pop()
            }
        };
        if let Some(id) = next_disk_id {
            let (size_rw, size_root_fs) = docker_sdk::list_of_containers::get_container_size(
                self.app.settings_model.docker_url.to_string(),
                &id,
            )
            .await;
            self.app
                .cache
                .update_disk_usage(&id, size_rw, size_root_fs)
                .await;
        }

        let mut usages_result = Vec::new();

        for container in list_of_containers {
            if container.is_running() {
                let container_id = container.id.to_string();
                let url = self.app.settings_model.docker_url.to_string();
                let proc_base = self.app.settings_model.host_proc_path().to_string();
                let statistics_task = tokio::spawn(async move {
                    let usage =
                        docker_sdk::sdk::get_container_stats(url.clone(), container_id.clone())
                            .await;

                    // Combined inspect → started_at + FD usage in one daemon RTT.
                    let probe =
                        crate::proc_fd::probe_container(&url, &proc_base, &container_id).await;

                    (container_id, usage, probe)
                });

                usages_result.push(statistics_task);
            } else {
                self.app.cache.reset_usage(&container.id).await;
            }
        }

        for usage_result in usages_result {
            let usage_result = usage_result.await;

            if usage_result.is_err() {
                continue;
            }

            let (container_id, usage_result, probe) = usage_result.unwrap();

            if let Some(usage) = usage_result {
                self.app
                    .cache
                    .update_usage(
                        &container_id,
                        usage.get_used_memory(),
                        usage.get_available_memory(),
                        usage.memory_stats.limit,
                        usage.get_cpu_usage(),
                        usage.total_rx_bytes(),
                        usage.total_tx_bytes(),
                    )
                    .await;
            } else {
                self.app.cache.reset_usage(&container_id).await;
            }

            // Set after the stats branch — `reset_usage` also clears fd fields,
            // so writing fd usage last keeps it intact even when stats failed.
            self.app
                .cache
                .update_fd_usage(&container_id, probe.open_files, probe.fd_limit)
                .await;
            self.app
                .cache
                .update_started_at(&container_id, probe.started_at_unix_seconds)
                .await;
        }

        println!("Iteration is finished in {}", sw.duration_as_string());
    }
}
