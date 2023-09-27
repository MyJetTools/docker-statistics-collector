use std::sync::Arc;

use rust_extensions::{MyTimerTick, StopWatch};

use crate::app::AppContext;

pub struct SyncContainersInfoTimer {
    app: Arc<AppContext>,
}

impl SyncContainersInfoTimer {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl MyTimerTick for SyncContainersInfoTimer {
    async fn tick(&self) {
        let mut sw = StopWatch::new();

        sw.start();
        let list_of_containers = docker_sdk::list_of_containers::get_list_of_containers(
            self.app.settings_model.url.to_string(),
        )
        .await;

        self.app.cache.update_services(&list_of_containers).await;

        for container in list_of_containers {
            let usage = docker_sdk::container_stats::get_container_stats(
                self.app.settings_model.url.to_string(),
                container.id.to_string(),
            )
            .await;

            if container.is_running() {
                if let Some(usage) = usage {
                    self.app
                        .cache
                        .update_usage(
                            &container.id,
                            usage.get_used_memory(),
                            usage.get_available_memory(),
                            usage.get_cpu_usage(),
                        )
                        .await;
                }
            } else {
                self.app.cache.reset_usage(&container.id).await;
            }
        }

        sw.pause();
        println!("Iteration is finished in {}", sw.duration_as_string());
    }
}
