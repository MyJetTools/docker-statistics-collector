#[tokio::main]
async fn main() {
    let url = "http://localhost:2375";

    /*
       let result = list_of_containers::get_list_of_containers(url).await;
       for itm in result {
           if itm.image.contains("signal") {
               println!("{:#?}", itm);
           }
       }
    */

    let stats = docker_sdk::container_stats::get_container_stats(
        url.to_string(),
        "49b9068b28ac68992712bd921cb56c32360275f1d3709399489b22b530edb87c".to_string(),
    )
    .await
    .unwrap();

    println!("{:#?}", stats);

    println!(
        "{}/{}",
        format_mem(stats.get_used_memory()),
        format_mem(stats.get_available_memory())
    );

    println!("{:?}%", stats.get_cpu_usage());
}

fn format_mem(mem: i64) -> String {
    let mem = mem as f64;
    if mem < 1024.0 {
        return format!("{}B", mem);
    }

    let mem = mem / 1024.0;

    if mem < 1024.0 {
        return format!("{}KB", mem);
    }

    let mem = mem / 1024.0;

    if mem < 1024.0 {
        return format!("{}MB", mem);
    }

    let mem = mem / 1024.0;

    return format!("{}GB", mem);
}
