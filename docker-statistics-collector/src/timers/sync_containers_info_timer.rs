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
        let sw = StopWatch::new();

        let list_of_containers = docker_sdk::list_of_containers::get_list_of_containers(
            self.app.settings_model.url.to_string(),
        )
        .await;

        self.app.cache.update_services(&list_of_containers).await;

        let mut usages_result = Vec::new();

        for container in list_of_containers {
            if container.is_running() {
                let container_id = container.id.to_string();
                let url = self.app.settings_model.url.to_string();
                let statistics_task = tokio::spawn(async move {
                    let usage =
                        docker_sdk::sdk::get_container_stats(url, container_id.to_string()).await;

                    (container_id, usage)
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

            let (container_id, usage_result) = usage_result.unwrap();

            if let Some(usage) = usage_result {
                self.app
                    .cache
                    .update_usage(
                        &container_id,
                        usage.get_used_memory(),
                        usage.get_available_memory(),
                        usage.memory_stats.limit,
                        usage.get_cpu_usage(),
                    )
                    .await;
            } else {
                self.app.cache.reset_usage(&container_id).await;
            }
        }

        println!("Iteration is finished in {}", sw.duration_as_string());
    }
}
