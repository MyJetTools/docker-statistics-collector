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
            if self
                .app
                .settings_model
                .ignore_service(service_name.as_str())
            {
                continue;
            }

            let url = format!("http://{}:{}/metrics", service_name, metrics_port);

            let metrics = FlUrl::new(url.as_str()).get().await;

            match metrics {
                Ok(metrics) => {
                    if metrics.get_status_code() == 200 {
                        if let Ok(body) = metrics.receive_body().await {
                            if is_prometheus_metrics_content(body.as_slice()) {
                                let injected_with_app =
                                    inject_app_name(body.as_slice(), service_name.as_str());
                                self.app
                                    .metrics_cache
                                    .update(service_name, injected_with_app)
                                    .await;
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Can not load metric from: {}. Error: {:?}", url, err);
                }
            }
        }

        let snapshot = self.app.metrics_cache.get_sizes().await;

        println!("{:#?}", snapshot);
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

fn inject_app_name(src: &[u8], app_name: &str) -> Vec<u8> {
    let mut result = Vec::new();

    let to_inject = format!("app=\"{}\",", app_name);

    for b in src {
        let b = *b;

        result.push(b);
        if b == b'{' {
            result.extend(to_inject.as_bytes());
        }
    }

    result
}
