use std::sync::Arc;

use flurl::FlUrl;
use rust_extensions::MyTimerTick;

use crate::app::AppContext;

pub struct SyncMetricsEndpointsTimer {
    app: Arc<AppContext>,
}

impl SyncMetricsEndpointsTimer {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl MyTimerTick for SyncMetricsEndpointsTimer {
    async fn tick(&self) {
        let snapshot = self.app.cache.get_snapshot().await;

        let mut service_names = Vec::new();
        for mut service_info in snapshot {
            if let Some(mut labels) = service_info.labels.take() {
                if let Some(value) = labels.remove("com.docker.compose.service") {
                    service_names.push(value);
                }
            }
        }

        for service_name in service_names {
            let url = format!(
                "http://{}:8000/metrics",
                self.app.settings_model.url.to_string()
            );

            let metrics = FlUrl::new(url.as_str()).get().await;

            match metrics {
                Ok(metrics) => {
                    if metrics.get_status_code() == 200 {
                        if let Ok(body) = metrics.receive_body().await {
                            self.app.metrics_cache.update(service_name, body).await;
                        }
                    }
                }
                Err(err) => {
                    println!("Can not load metric from: {}. Error: {:?}", url, err);
                }
            }
        }
    }
}
