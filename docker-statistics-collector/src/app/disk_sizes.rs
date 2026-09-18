use std::collections::BTreeMap;

use tokio::sync::RwLock;

/// Disk usage of one container, in bytes.
#[derive(Clone, Copy, Default)]
pub struct DiskSize {
    /// Writable-layer size — the container's own data on top of the image.
    pub size_rw: Option<i64>,
    /// Total size including the read-only image layers.
    pub size_root_fs: Option<i64>,
}

/// The one thing the collector deliberately DOES remember.
///
/// Everything else here is read live per request, but Docker has to walk the storage
/// layers to size a container — seconds per container — so measuring on demand would put
/// that cost on the request path and make every poll wait for it. Instead a timer
/// measures one container at a time in the background and parks the answer here; each
/// containers payload then carries whatever is currently known, and the API service just
/// reads it along with everything else.
pub struct DiskSizesCache {
    data: RwLock<BTreeMap<String, DiskSize>>,
}

impl DiskSizesCache {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(BTreeMap::new()),
        }
    }

    /// Whole map in one lock — a scan asks for every container it just listed, and
    /// taking the lock once per snapshot beats taking it once per container.
    pub async fn get_snapshot(&self) -> BTreeMap<String, DiskSize> {
        self.data.read().await.clone()
    }

    pub async fn set(&self, container_id: &str, size_rw: Option<i64>, size_root_fs: Option<i64>) {
        // A measurement that produced nothing is not worth storing over a good one:
        // Docker occasionally answers without the size fields, and overwriting would
        // blank a Disk cell that was already correct.
        if size_rw.is_none() && size_root_fs.is_none() {
            return;
        }

        self.data.write().await.insert(
            container_id.to_string(),
            DiskSize {
                size_rw,
                size_root_fs,
            },
        );
    }

    /// Forget containers that no longer exist, so the map cannot outgrow the host it
    /// describes on a box with heavy container churn.
    pub async fn retain(&self, alive_ids: &[String]) {
        let mut write_access = self.data.write().await;
        write_access.retain(|id, _| alive_ids.iter().any(|alive| alive == id));
    }
}
