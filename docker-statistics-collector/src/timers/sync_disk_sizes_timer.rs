use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rust_extensions::{MyTimerTick, RepeatTimerIteration};

use crate::app::AppContext;

/// Per-container disk usage is expensive (Docker walks the storage layers), so we never
/// compute the whole batch at once. Every Nth *idle* tick we refill `pending` with all
/// current container ids, then drain it one container per tick. Ticks that drain the
/// queue do NOT count toward the next refill.
const REFILL_EVERY_N_IDLE_TICKS: u64 = 10;

/// The collector's only background timer, and the only thing it keeps in memory.
///
/// Sizing a container is far too slow to do on the request path, so it happens here and
/// the answer is parked in [`crate::app::DiskSizesCache`]. Every containers payload then
/// carries whatever is known at that moment — the API service simply reads it along with
/// the rest, on its own polling schedule, and does no measuring of its own.
pub struct SyncDiskSizesTimer {
    app: Arc<AppContext>,
    /// Container ids still waiting for a disk-size pass (drained one per tick).
    pending: Mutex<Vec<String>>,
    /// Idle ticks (queue empty) counted toward the next refill.
    idle_tick_no: AtomicU64,
}

impl SyncDiskSizesTimer {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self {
            app,
            pending: Mutex::new(Vec::new()),
            idle_tick_no: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl MyTimerTick for SyncDiskSizesTimer {
    async fn tick(&self) -> RepeatTimerIteration {
        // The plain listing, not a full stats scan — all this needs is the set of ids.
        let containers =
            match docker_sdk::list_of_containers::get_list_of_containers(
                self.app.settings_model.docker_url.to_string(),
            )
            .await
            {
                Ok(containers) => containers,
                Err(err) => {
                    eprintln!("SyncDiskSizesTimer: cannot list containers: {}", err);
                    return RepeatTimerIteration::WithInterval;
                }
            };

        let alive_ids: Vec<String> = containers.iter().map(|c| c.id.clone()).collect();
        self.app.disk_sizes.retain(&alive_ids).await;

        let next_id = {
            let mut pending = self.pending.lock().unwrap();
            if pending.is_empty() {
                let n = self.idle_tick_no.fetch_add(1, Ordering::Relaxed) + 1;
                if n >= REFILL_EVERY_N_IDLE_TICKS {
                    self.idle_tick_no.store(0, Ordering::Relaxed);
                    *pending = alive_ids;
                }
            }
            pending.pop()
        };

        let Some(id) = next_id else {
            return RepeatTimerIteration::WithInterval;
        };

        let (size_rw, size_root_fs) = docker_sdk::list_of_containers::get_container_size(
            self.app.settings_model.docker_url.to_string(),
            &id,
        )
        .await;

        self.app.disk_sizes.set(&id, size_rw, size_root_fs).await;

        RepeatTimerIteration::WithInterval
    }
}
