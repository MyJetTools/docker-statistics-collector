/// A kind of measurement kept for a service. Each has a day file of its own, and the files
/// are all of one format - what differs is what the value and the limit of a record are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Metric {
    /// Cores in use, as the collector reports them; the limit is the cores of the host
    Cpu,
    /// Bytes of memory in use; the limit is the memory limit of the container
    Mem,
    /// Open file descriptors; the limit is the `nofile` limit
    Files,
    /// Inbound throughput, MB/s; it has no limit
    NetIn,
    /// Outbound throughput, MB/s; it has no limit
    NetOut,
}

impl Metric {
    pub const ALL: [Metric; 5] = [
        Metric::Cpu,
        Metric::Mem,
        Metric::Files,
        Metric::NetIn,
        Metric::NetOut,
    ];

    /// Its name in the api, and the extension of its day files
    pub fn as_str(&self) -> &'static str {
        match self {
            Metric::Cpu => "cpu",
            Metric::Mem => "mem",
            Metric::Files => "files",
            Metric::NetIn => "net-in",
            Metric::NetOut => "net-out",
        }
    }

    pub fn parse(src: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|metric| metric.as_str() == src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_metric_is_read_back_from_its_name() {
        for metric in Metric::ALL {
            assert_eq!(Metric::parse(metric.as_str()), Some(metric));
        }

        assert_eq!(Metric::parse("disk"), None);
        assert_eq!(Metric::parse(""), None);
    }
}
