use std::collections::HashMap;
use std::time::Duration;

use flurl::{
    hyper::header::{CONNECTION, HOST},
    IntoFlUrl,
};
use serde::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerStatsJsonModel {
    pub read: String,
    pub memory_stats: MemoryStatsJsonModel,
    pub cpu_stats: CpuStatsJsonModel,
    pub precpu_stats: CpuStatsJsonModel,
    /// Per-interface cumulative byte counters. Absent for containers on
    /// `network_mode: none/host` — treated as zero traffic.
    #[serde(default)]
    pub networks: Option<HashMap<String, NetworkStatJsonModel>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkStatJsonModel {
    #[serde(default)]
    pub rx_bytes: i64,
    #[serde(default)]
    pub tx_bytes: i64,
}

impl ContainerStatsJsonModel {
    pub fn get_available_memory(&self) -> i64 {
        self.memory_stats.limit
    }

    pub fn get_used_memory(&self) -> i64 {
        if let Some(total_inactive_file) = self.memory_stats.stats.total_inactive_file {
            return self.memory_stats.usage - total_inactive_file;
        }

        if let Some(inactive_file) = self.memory_stats.stats.inactive_file {
            return self.memory_stats.usage - inactive_file;
        }

        if let Some(cache) = self.memory_stats.stats.cache {
            return self.memory_stats.usage - cache;
        }

        self.memory_stats.usage

        //self.memory_stats.usage
    }

    pub fn cpu_delta(&self) -> i64 {
        self.cpu_stats.cpu_usage.total_usage - self.precpu_stats.cpu_usage.total_usage
    }

    pub fn system_cpu_delta(&self) -> i64 {
        self.cpu_stats.system_cpu_usage - self.precpu_stats.system_cpu_usage
    }

    pub fn number_cpus(&self) -> i64 {
        self.cpu_stats.online_cpus
    }

    pub fn get_cpu_usage(&self) -> f64 {
        let cpu_delta = self.cpu_delta() as f64;
        let system_cpu_delta = self.system_cpu_delta() as f64;

        let result = (cpu_delta / system_cpu_delta) * self.number_cpus() as f64;

        result
    }

    /// Total received bytes across all interfaces (cumulative since container
    /// start). Rate is derived in the collector from the delta between polls.
    pub fn total_rx_bytes(&self) -> i64 {
        match self.networks.as_ref() {
            Some(nets) => nets.values().map(|n| n.rx_bytes).sum(),
            None => 0,
        }
    }

    /// Total transmitted bytes across all interfaces (cumulative).
    pub fn total_tx_bytes(&self) -> i64 {
        match self.networks.as_ref() {
            Some(nets) => nets.values().map(|n| n.tx_bytes).sum(),
            None => 0,
        }
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
    pub cache: Option<i64>,
    pub total_inactive_file: Option<i64>,
    pub inactive_file: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CpuStatsJsonModel {
    pub system_cpu_usage: i64,
    pub cpu_usage: CpuUsageJsonModel,
    pub online_cpus: i64,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CpuUsageJsonModel {
    pub total_usage: i64,
}

pub async fn get_container_stats(
    url: String,
    container_id: String,
) -> Option<ContainerStatsJsonModel> {
    let mut response = url
        .as_str()
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("stats")
        .with_header("host", "localhost")
        .append_query_param("stream", Some("false"))
        .set_timeout(Duration::from_secs(5))
        .get()
        .await
        .unwrap();

    if response.get_status_code() != 200 {
        println!("url: {}", url);
        println!("Status code: {}", response.get_status_code());
        println!("Headers: {:#?}", response.get_headers());
    }

    let response = response.get_body_as_slice().await.unwrap();
    let result = serde_json::from_slice(response);

    if let Err(err) = &result {
        println!("Err:{}", err);
        println!("{}", std::str::from_utf8(response).unwrap());
        return None;
    }

    Some(result.unwrap())
}

pub async fn get_container_logs(url: &str, container_id: &str, last_lines_number: u32) -> Vec<u8> {
    let response = url
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("logs")
        .append_query_param("stdout", Some("1"))
        .append_query_param("stderr", Some("1"))
        .append_query_param("timestamps", Some("1"))
        .append_query_param("tail", Some(last_lines_number.to_string()))
        .with_header(HOST.as_str(), "docker")
        .with_header(CONNECTION.as_str(), "close")
        .set_timeout(Duration::from_secs(5))
        //  .print_input_request()
        .get()
        .await;

    if let Err(err) = &response {
        print!("get_container_logs Err:{:? }", err);
        panic!("{:?}", err);
    }

    let response = response.unwrap();

    //  println!("url: {}", url);
    //  println!("Status code: {}", response.get_status_code());
    //  println!("Headers: {:#?}", response.get_headers());

    let body = response.receive_body().await.unwrap();

    //    println!("Body Len: {}", body.len());

    body
}

/// Open a streaming connection to `docker logs --follow` for one container.
/// Caller drives it with `get_next_chunk().await` and is responsible for
/// dropping the stream to tear down the docker side. The stream yields raw
/// bytes in the docker multiplexed log frame format
/// (`[stream_type, 0, 0, 0, size_be4, payload...]`) — same as
/// `get_container_logs`, just continuous.
///
/// `tail` controls the initial backfill (number of lines from the past sent
/// before the follow stream starts).
pub async fn get_container_logs_stream(
    url: &str,
    container_id: &str,
    tail: u32,
) -> Result<flurl::FlResponseAsStream, flurl::FlUrlError> {
    let response = url
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("logs")
        .append_query_param("stdout", Some("1"))
        .append_query_param("stderr", Some("1"))
        .append_query_param("timestamps", Some("1"))
        .append_query_param("follow", Some("1"))
        .append_query_param("tail", Some(tail.to_string()))
        .with_header(HOST.as_str(), "docker")
        // Hold the connection open for the whole follow lifetime.
        .with_header(CONNECTION.as_str(), "keep-alive")
        .get()
        .await?;

    Ok(response.get_body_as_stream())
}
