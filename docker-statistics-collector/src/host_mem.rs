/// Host snapshot read from `<proc_base>` — memory + CPU count.
///
/// Memory values are in bytes; `cpu_count` is the number of logical processors.
/// A missing `MemAvailable` field (very old kernels) falls back to
/// `MemFree + Buffers + Cached + SReclaimable`. `None` from [`read`] means
/// `/proc/meminfo` could not be read at all — `/proc` not bind-mounted, or
/// the platform has no `/proc` (macOS). `cpu_count` is independently optional
/// (may be `None` if memory was read but `cpuinfo` wasn't).
#[derive(Clone, Copy, Debug)]
pub struct HostMemSnapshot {
    pub total: i64,
    pub available: i64,
    pub used: i64,
    pub cpu_count: Option<u32>,
}

pub fn read(proc_base: &str) -> Option<HostMemSnapshot> {
    let mem_content = std::fs::read_to_string(format!("{}/meminfo", proc_base)).ok()?;
    let mut snap = parse(&mem_content)?;
    snap.cpu_count = read_cpu_count(proc_base);
    Some(snap)
}

fn read_cpu_count(proc_base: &str) -> Option<u32> {
    let content = std::fs::read_to_string(format!("{}/cpuinfo", proc_base)).ok()?;
    let count = content
        .lines()
        .filter(|l| l.starts_with("processor"))
        .count();
    if count == 0 {
        None
    } else {
        Some(count as u32)
    }
}

fn parse(content: &str) -> Option<HostMemSnapshot> {
    let mut total_kb: Option<i64> = None;
    let mut available_kb: Option<i64> = None;
    let mut free_kb: Option<i64> = None;
    let mut buffers_kb: Option<i64> = None;
    let mut cached_kb: Option<i64> = None;
    let mut sreclaimable_kb: Option<i64> = None;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available_kb = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("MemFree:") {
            free_kb = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("Buffers:") {
            buffers_kb = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("Cached:") {
            cached_kb = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("SReclaimable:") {
            sreclaimable_kb = parse_kb(rest);
        }
    }

    let total = total_kb?;
    let available = match available_kb {
        Some(v) => v,
        None => {
            // Pre-3.14 kernels — best-effort estimate.
            free_kb.unwrap_or(0)
                + buffers_kb.unwrap_or(0)
                + cached_kb.unwrap_or(0)
                + sreclaimable_kb.unwrap_or(0)
        }
    };
    let used = (total - available).max(0);

    Some(HostMemSnapshot {
        total: total * 1024,
        available: available * 1024,
        used: used * 1024,
        cpu_count: None,
    })
}

fn parse_kb(rest: &str) -> Option<i64> {
    rest.split_whitespace().next()?.parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_meminfo() {
        let sample = "MemTotal:       16384000 kB\n\
                      MemFree:         1024000 kB\n\
                      MemAvailable:    8192000 kB\n\
                      Buffers:          200000 kB\n\
                      Cached:          4000000 kB\n\
                      SReclaimable:     300000 kB\n";
        let s = parse(sample).unwrap();
        assert_eq!(s.total, 16384000_i64 * 1024);
        assert_eq!(s.available, 8192000_i64 * 1024);
        assert_eq!(s.used, (16384000_i64 - 8192000) * 1024);
        // parse() never reads cpuinfo; that's the read()-layer job.
        assert!(s.cpu_count.is_none());
    }

    #[test]
    fn fallback_when_no_memavailable() {
        let sample = "MemTotal:       8192000 kB\n\
                      MemFree:         512000 kB\n\
                      Buffers:         100000 kB\n\
                      Cached:         2000000 kB\n\
                      SReclaimable:    150000 kB\n";
        let s = parse(sample).unwrap();
        let expected_available = 512000 + 100000 + 2000000 + 150000;
        assert_eq!(s.available, expected_available * 1024);
    }
}
