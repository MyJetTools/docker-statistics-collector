use std::collections::HashMap;

use rust_extensions::date_time::DateTimeAsMicroseconds;
use tokio::sync::RwLock;

use super::ServiceInfo;

#[derive(Clone, Debug)]
pub struct PeerSnapshot {
    pub instance: String,
    pub peer_url: String,
    #[allow(dead_code)]
    pub fetched_at: DateTimeAsMicroseconds,
    pub containers: Vec<ServiceInfo>,
    #[allow(dead_code)]
    pub last_error: Option<String>,
}

pub struct PeersCache {
    pub data: RwLock<HashMap<String, PeerSnapshot>>,
}

impl PeersCache {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    pub async fn update(
        &self,
        peer_url: &str,
        instance: String,
        containers: Vec<ServiceInfo>,
    ) {
        let mut write_access = self.data.write().await;
        write_access.insert(
            peer_url.to_string(),
            PeerSnapshot {
                instance,
                peer_url: peer_url.to_string(),
                fetched_at: DateTimeAsMicroseconds::now(),
                containers,
                last_error: None,
            },
        );
    }

    pub async fn record_error(&self, peer_url: &str, err: String) {
        let mut write_access = self.data.write().await;
        match write_access.get_mut(peer_url) {
            Some(snapshot) => {
                snapshot.last_error = Some(err);
            }
            None => {
                write_access.insert(
                    peer_url.to_string(),
                    PeerSnapshot {
                        instance: String::new(),
                        peer_url: peer_url.to_string(),
                        fetched_at: DateTimeAsMicroseconds::now(),
                        containers: Vec::new(),
                        last_error: Some(err),
                    },
                );
            }
        }
    }

    pub async fn get_snapshot(&self) -> Vec<PeerSnapshot> {
        let read_access = self.data.read().await;
        read_access.values().cloned().collect()
    }

    pub async fn find_peer_for_container(&self, container_id: &str) -> Option<(String, String)> {
        let read_access = self.data.read().await;
        for snapshot in read_access.values() {
            if snapshot
                .containers
                .iter()
                .any(|c| c.id == container_id)
            {
                return Some((snapshot.peer_url.clone(), snapshot.instance.clone()));
            }
        }
        None
    }
}
