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
        .append_query_param("stream", Some("false"))
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

pub async fn get_container_logs(
    url: String,
    container_id: String,
    last_lines_number: u32,
) -> Vec<u8> {
    let response = url
        .as_str()
        .append_path_segment("containers")
        .append_path_segment(container_id)
        .append_path_segment("logs")
        .append_query_param("stdout", Some("true"))
        .append_query_param("timestamps", Some("true"))
        .append_query_param("tail", Some(last_lines_number.to_string()))
        .get()
        .await;

    if let Err(err) = &response {
        print!("get_container_logs Err:{:? }", err);
        panic!("{:?}", err);
    }

    let response = response.unwrap();

    println!("url: {}", url);
    println!("Status code: {}", response.get_status_code());
    println!("Headers: {:#?}", response.get_headers());

    response.receive_body().await.unwrap()
}
