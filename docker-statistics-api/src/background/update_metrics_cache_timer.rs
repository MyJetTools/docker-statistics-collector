use std::collections::BTreeMap;

use rust_extensions::MyTimerTick;

use crate::app::DataCache;
use crate::APP_CTX;

pub struct UpdateMetricsCacheTimer;

#[async_trait::async_trait]
impl MyTimerTick for UpdateMetricsCacheTimer {
    async fn tick(&self) {
        let stop_watch = rust_extensions::StopWatch::new();

        let urls = APP_CTX.settings_reader.get_urls().await;

        let mut spawns = Vec::new();
        for (env, master_url) in urls {
            let task = tokio::spawn(async move {
                let fl_url = APP_CTX.create_fl_url(&master_url);

                let statistics = crate::http_client::get_statistics(fl_url).await;

                let statistics = match statistics {
                    Ok(s) => s,
                    Err(err) => {
                        println!(
                            "Failed to get statistics for env {}. Master url: {}. Err: {:?}",
                            env, master_url, err
                        );
                        return;
                    }
                };

                let host_mem_by_instance = DataCache::host_mem_map(&statistics.hosts);

                let mut by_instance: BTreeMap<String, Vec<_>> = BTreeMap::new();
                for container in statistics.containers {
                    by_instance
                        .entry(container.instance.clone())
                        .or_default()
                        .push(container);
                }

                if by_instance.is_empty() {
                    println!(
                        "env {} master {} returned 0 containers — clearing cache for this env",
                        env, master_url
                    );
                }

                {
                    let mut data_cache = APP_CTX.data_cache_by_env.lock().await;
                    data_cache.update_from_master(
                        &env,
                        by_instance,
                        host_mem_by_instance,
                        master_url,
                    );
                }
            });

            spawns.push(task);
        }

        for spawn in spawns {
            let _ = spawn.await;
        }

        println!("UpdateMetricsCacheTimer took: {:?}", stop_watch.duration());
    }
}
