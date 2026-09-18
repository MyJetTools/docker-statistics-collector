use flurl::FlUrl;

use crate::app::AppContext;

/// On-demand Prometheus scrape.
///
/// The collector no longer keeps a metrics cache and no longer runs a scraping
/// timer — it scrapes when someone asks and forgets the result immediately.
/// It still has to be the one doing the HTTP call: the endpoints live at
/// `http://<compose-service>:<metrics_port>/metrics`, which only resolves from
/// inside that VM's Docker network, so a central service cannot reach them.
pub struct ScrapedService {
    pub service_name: String,
    pub content: Vec<u8>,
}

/// Scrape every compose service on this host in parallel. Services that don't
/// answer, answer non-200, or don't return Prometheus text are skipped.
pub async fn scrape_all(app: &AppContext) -> Vec<ScrapedService> {
    if app.settings_model.metrics_collecting_disabled() {
        return Vec::new();
    }

    let containers = match crate::app::read_container_list(&app.settings_model).await {
        Ok(containers) => containers,
        Err(err) => {
            eprintln!("metrics_scraper: cannot list containers: {}", err);
            return Vec::new();
        }
    };

    let mut service_names = Vec::new();
    for container in containers.iter() {
        let Some(service_name) = container.compose_service() else {
            continue;
        };
        if app.settings_model.ignore_service(service_name) {
            continue;
        }
        if service_names.iter().any(|itm: &String| itm == service_name) {
            continue;
        }
        service_names.push(service_name.to_string());
    }

    // Sorted so `/metrics` output is byte-stable between calls regardless of
    // the order Docker happened to list the containers in.
    service_names.sort();

    let metrics_port = app.settings_model.metrics_port;

    let mut tasks = Vec::with_capacity(service_names.len());
    for service_name in service_names {
        tasks.push(tokio::spawn(async move {
            scrape_one(&service_name, metrics_port)
                .await
                .map(|content| ScrapedService {
                    service_name,
                    content,
                })
        }));
    }

    let mut result = Vec::new();
    for task in tasks {
        if let Ok(Some(scraped)) = task.await {
            result.push(scraped);
        }
    }

    result
}

/// Scrape a single compose service by name. `None` when it didn't answer with
/// Prometheus text.
pub async fn scrape_service(app: &AppContext, service_name: &str) -> Option<Vec<u8>> {
    if app.settings_model.metrics_collecting_disabled() {
        return None;
    }

    if app.settings_model.ignore_service(service_name) {
        return None;
    }

    scrape_one(service_name, app.settings_model.metrics_port).await
}

/// Per-target budget. `/metrics` now costs the SLOWEST exporter instead of a cache
/// read, and Prometheus gives the whole page 10s by default — so one exporter that
/// accepts the connection and goes quiet must not be able to blank every other service
/// on the host.
const SCRAPE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

async fn scrape_one(service_name: &str, metrics_port: u16) -> Option<Vec<u8>> {
    let url = format!("http://{}:{}/metrics", service_name, metrics_port);

    let response = match FlUrl::new(url.as_str())
        .set_timeout(SCRAPE_TIMEOUT)
        .set_response_body_timeout(SCRAPE_TIMEOUT)
        .get()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            eprintln!("Can not load metric from: {}. Error: {:?}", url, err);
            return None;
        }
    };

    if response.get_status_code() != 200 {
        return None;
    }

    let body = response.receive_body().await.ok()?;

    if !is_prometheus_metrics_content(body.as_slice()) {
        return None;
    }

    Some(inject_app_name(body.as_slice(), service_name))
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
