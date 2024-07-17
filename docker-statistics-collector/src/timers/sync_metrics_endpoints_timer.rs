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
        if let Some(disabled) = self.app.settings_model.disable_metics_collecting {
            if disabled {
                return;
            }
        }

        let snapshot = self.app.cache.get_snapshot().await;

        let mut service_names = Vec::new();
        for mut service_info in snapshot {
            if let Some(mut labels) = service_info.labels.take() {
                if let Some(value) = labels.remove("com.docker.compose.service") {
                    service_names.push(value);
                }
            }
        }

        let metrics_port = self.app.settings_model.metrics_port;

        for service_name in service_names {
            let url = format!("http://{}:{}/metrics", service_name, metrics_port);

            let metrics = FlUrl::new(url.as_str())
                .do_not_reuse_connection()
                .get()
                .await;

            match metrics {
                Ok(metrics) => {
                    if metrics.get_status_code() == 200 {
                        if let Ok(body) = metrics.receive_body().await {
                            if is_prometheus_metrics_content(body.as_slice()) {
                                self.app.metrics_cache.update(service_name, body).await;
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("Can not load metric from: {}. Error: {:?}", url, err);
                }
            }
        }

        let size = self.app.metrics_cache.get_content_size().await;

        println!(
            "Now metrics content size is: {}. Amount of records: {}",
            size.1, size.0
        );
    }
}

fn is_prometheus_metrics_content(src: &[u8]) -> bool {
    for b in src {
        let b = *b;

        if b <= 32 {
            continue;
        }

        return b == b'#';
    }

    false
}
