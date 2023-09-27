use std::sync::atomic::AtomicBool;

use flurl::IntoFlUrl;
use serde::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerStatsJsonModel {
    pub read: String,
    pub memory_stats: MemoryStatsJsonModel,
    pub cpu_stats: CpuStatsJsonModel,
    pub precpu_stats: CpuStatsJsonModel,
}

impl ContainerStatsJsonModel {
    pub fn get_available_memory(&self) -> i64 {
        self.memory_stats.limit
    }

    pub fn get_used_memory(&self) -> i64 {
        self.memory_stats.usage - self.memory_stats.stats.cache
        //self.memory_stats.usage
    }

    pub fn cpu_delta(&self) -> i64 {
        self.cpu_stats.cpu_usage.total_usage - self.precpu_stats.cpu_usage.total_usage
    }

    pub fn system_cpu_delta(&self) -> Option<i64> {
        let result = self.cpu_stats.system_cpu_usage? - self.precpu_stats.system_cpu_usage?;

        Some(result)
    }

    pub fn number_cpus(&self) -> Option<i64> {
        self.cpu_stats.online_cpus
    }

    pub fn get_cpu_usage(&self) -> Option<f64> {
        let cpu_delta = self.cpu_delta() as f64;
        let system_cpu_delta = self.system_cpu_delta()? as f64;

        let result = (cpu_delta / system_cpu_delta) * self.number_cpus()? as f64 * 100.0;

        Some(result)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MemoryStatsJsonModel {
    pub limit: i64,
    pub usage: i64,
    pub stats: MemoryStatsData,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MemoryStatsData {
    pub cache: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CpuStatsJsonModel {
    pub system_cpu_usage: Option<i64>,
    pub cpu_usage: CpuUsageJsonModel,
    pub online_cpus: Option<i64>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CpuUsageJsonModel {
    pub total_usage: i64,
}

static MADE_PRINT: AtomicBool = AtomicBool::new(false);

pub async fn get_container_stats(url: String, container_id: String) -> ContainerStatsJsonModel {
    let mut response = url
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("stats")
        .append_query_param("stream", Some("false"))
        .get()
        .await
        .unwrap();

    let result = response.get_body().await.unwrap();

    if !MADE_PRINT.load(std::sync::atomic::Ordering::Relaxed) {
        println!("{:?}", std::str::from_utf8(result).unwrap());
        MADE_PRINT.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    serde_json::from_slice(&result).unwrap()
}
