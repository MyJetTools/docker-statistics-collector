pub fn format_mem(mem: i64) -> String {
    let mem = mem as f64;
    if mem < 1024.0 {
        return format!("{}B", mem);
    }

    let mem = mem / 1024.0;

    if mem < 1024.0 {
        return format!("{:.2}KB", mem);
    }

    let mem = mem / 1024.0;

    if mem < 1024.0 {
        return format!("{:.2}MB", mem);
    }

    let mem = mem / 1024.0;

    return format!("{:.2}GB", mem);
}
