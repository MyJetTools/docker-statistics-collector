use crate::models::{ContainerJsonModel, NetUsageJsonMode};

use super::Metric;

/// A stamp further than this from the clock of this service is taken for a wrong one
const MAX_CLOCK_SKEW_SECONDS: i64 = 24 * 60 * 60;

/// A container with no name goes by the start of its id, as it does in the UI
const ID_AS_NAME_LEN: usize = 12;

/// One value a reading of a container took
#[derive(Debug, Clone, Copy)]
pub struct MetricValue {
    pub metric: Metric,
    pub value: f64,
    /// NaN when the value has no limit
    pub limit: f64,
}

/// What one reading of a container measured: its values, all of the second the collector
/// took them at. The storage keeps it as a record per value.
#[derive(Debug, Clone)]
pub struct ServiceSample {
    pub vm: String,
    /// The name of the container. Its id changes with every release and its name does
    /// not, so the history of a service goes on through its releases.
    pub service: String,
    /// Unix seconds
    pub time: u32,
    pub values: Vec<MetricValue>,
}

impl ServiceSample {
    /// `None` when the reading measured nothing - a container which is not running, or
    /// one whose stats did not read.
    ///
    /// `net` is the throughput derived for this reading. `host_cpu_count` is what the cpu
    /// is measured against: the collector reports the cores in use, and does not report a
    /// cpu limit of the container.
    pub fn of_container(
        vm: &str,
        container: &ContainerJsonModel,
        net: &NetUsageJsonMode,
        host_cpu_count: Option<u32>,
        now_unix_seconds: i64,
    ) -> Option<Self> {
        let mut values = Vec::with_capacity(Metric::ALL.len());

        let mut add = |metric: Metric, value: Option<f64>, limit: Option<f64>| {
            let Some(value) = value.filter(|value| value.is_finite()) else {
                return;
            };

            let limit = limit
                .filter(|limit| limit.is_finite() && *limit > 0.0)
                .unwrap_or(f64::NAN);

            values.push(MetricValue {
                metric,
                value,
                limit,
            });
        };

        add(
            Metric::Cpu,
            container.cpu.usage,
            host_cpu_count.map(|count| count as f64),
        );
        add(
            Metric::Mem,
            container.mem.usage.map(|usage| usage as f64),
            container.mem.limit.map(|limit| limit as f64),
        );
        add(
            Metric::Files,
            container.files.open.map(|open| open as f64),
            container.files.limit.map(|limit| limit as f64),
        );
        add(Metric::NetIn, net.in_mbps, None);
        add(Metric::NetOut, net.out_mbps, None);

        if values.is_empty() {
            return None;
        }

        Some(Self {
            vm: vm.to_string(),
            service: service_name(container),
            time: sample_time(container.net.sampled_at_unix_ms, now_unix_seconds)?,
            values,
        })
    }
}

/// What a machine as a whole is kept under, next to its services: a name no container can
/// have - a Docker name starts with a letter or a digit.
pub const VM_SERIES: &str = "*";

/// What the containers of one VM come to in one poll: the figures the card of the machine
/// shows, kept as one more series next to those of its services.
#[derive(Debug, Default)]
pub struct VmTotals {
    cpu: Option<f64>,
    mem: Option<f64>,
    files: Option<f64>,
    net_in: Option<f64>,
    net_out: Option<f64>,
}

impl VmTotals {
    /// A container of the VM, and the throughput derived for this reading of it. What a
    /// container did not measure adds nothing - and a total nothing was added to is not a
    /// zero, it is not there.
    pub fn add(&mut self, container: &ContainerJsonModel, net: &NetUsageJsonMode) {
        add_to(&mut self.cpu, container.cpu.usage);
        add_to(&mut self.mem, container.mem.usage.map(|usage| usage as f64));
        add_to(
            &mut self.files,
            container.files.open.map(|open| open as f64),
        );
        add_to(&mut self.net_in, net.in_mbps);
        add_to(&mut self.net_out, net.out_mbps);
    }

    /// `None` when no container of the VM measured anything.
    ///
    /// The cpu of a machine is measured against its cores and its memory against its RAM;
    /// open files and throughput add up to nothing that has a limit. The time is the one
    /// the poll came at: the readings of the containers are not of one second.
    pub fn into_sample(
        self,
        vm: &str,
        host_cpu_count: Option<u32>,
        host_mem_total: Option<i64>,
        now_unix_seconds: i64,
    ) -> Option<ServiceSample> {
        let limit_of = |limit: Option<f64>| {
            limit
                .filter(|limit| limit.is_finite() && *limit > 0.0)
                .unwrap_or(f64::NAN)
        };

        let values: Vec<MetricValue> = [
            (
                Metric::Cpu,
                self.cpu,
                host_cpu_count.map(|count| count as f64),
            ),
            (
                Metric::Mem,
                self.mem,
                host_mem_total.map(|total| total as f64),
            ),
            (Metric::Files, self.files, None),
            (Metric::NetIn, self.net_in, None),
            (Metric::NetOut, self.net_out, None),
        ]
        .into_iter()
        .filter_map(|(metric, value, limit)| {
            Some(MetricValue {
                metric,
                value: value?,
                limit: limit_of(limit),
            })
        })
        .collect();

        if values.is_empty() {
            return None;
        }

        Some(ServiceSample {
            vm: vm.to_string(),
            service: VM_SERIES.to_string(),
            time: u32::try_from(now_unix_seconds)
                .ok()
                .filter(|seconds| *seconds > 0)?,
            values,
        })
    }
}

fn add_to(total: &mut Option<f64>, value: Option<f64>) {
    if let Some(value) = value.filter(|value| value.is_finite()) {
        *total = Some(total.unwrap_or(0.0) + value);
    }
}

/// The second the reading was taken at: the stamp the collector puts next to the reading.
/// A stamp which is not there - a collector older than the stamp - or is a day away from
/// the clock of this service is replaced by the time the reading came.
fn sample_time(sampled_at_unix_ms: i64, now_unix_seconds: i64) -> Option<u32> {
    let stamped = sampled_at_unix_ms.div_euclid(1000);

    let seconds =
        if sampled_at_unix_ms <= 0 || (stamped - now_unix_seconds).abs() > MAX_CLOCK_SKEW_SECONDS {
            now_unix_seconds
        } else {
            stamped
        };

    u32::try_from(seconds).ok().filter(|seconds| *seconds > 0)
}

/// What the service is kept under: the first name of the container without its slash -
/// the name the UI shows it by.
fn service_name(container: &ContainerJsonModel) -> String {
    match container
        .names
        .first()
        .map(|name| name.trim_start_matches('/'))
    {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => container.id.chars().take(ID_AS_NAME_LEN).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_791_800_000;

    fn container(json: serde_json::Value) -> ContainerJsonModel {
        serde_json::from_value(json).unwrap()
    }

    fn running() -> serde_json::Value {
        serde_json::json!({
            "id": "0123456789abcdef0123",
            "image": "api:1.0.0",
            "names": ["/api"],
            "labels": null,
            "enabled": true,
            "created": 1,
            "state": "running",
            "status": "Up",
            "cpu": { "usage": 0.37 },
            "mem": { "usage": 200, "available": 800, "limit": 1000 },
            "files": { "open": 42, "limit": 1024 },
            "net": { "rx_bytes": 1, "tx_bytes": 2, "sampled_at_unix_ms": (NOW - 2) * 1000 + 870 },
        })
    }

    fn net(in_mbps: f64, out_mbps: f64) -> NetUsageJsonMode {
        NetUsageJsonMode {
            in_mbps: Some(in_mbps),
            out_mbps: Some(out_mbps),
        }
    }

    fn value_of(sample: &ServiceSample, metric: Metric) -> Option<(f64, f64)> {
        sample
            .values
            .iter()
            .find(|value| value.metric == metric)
            .map(|value| (value.value, value.limit))
    }

    #[test]
    fn a_reading_gives_a_value_and_a_limit_per_metric() {
        let sample = ServiceSample::of_container(
            "vm-1",
            &container(running()),
            &net(1.5, 0.25),
            Some(8),
            NOW,
        )
        .unwrap();

        assert_eq!(sample.vm, "vm-1");
        assert_eq!(sample.service, "api");
        // The second the collector read it at - not the one it came at
        assert_eq!(sample.time as i64, NOW - 2);

        assert_eq!(value_of(&sample, Metric::Cpu), Some((0.37, 8.0)));
        assert_eq!(value_of(&sample, Metric::Mem), Some((200.0, 1000.0)));
        assert_eq!(value_of(&sample, Metric::Files), Some((42.0, 1024.0)));

        let (net_in, net_in_limit) = value_of(&sample, Metric::NetIn).unwrap();
        assert_eq!(net_in, 1.5);
        assert!(net_in_limit.is_nan());
        assert_eq!(value_of(&sample, Metric::NetOut).unwrap().0, 0.25);
    }

    #[test]
    fn what_was_not_measured_is_not_a_value() {
        let mut json = running();
        json["cpu"]["usage"] = serde_json::Value::Null;
        json["mem"]["limit"] = serde_json::Value::Null;

        // The first reading of a container: one sample is not a rate yet
        let sample = ServiceSample::of_container(
            "vm-1",
            &container(json),
            &NetUsageJsonMode::default(),
            None,
            NOW,
        )
        .unwrap();

        assert!(value_of(&sample, Metric::Cpu).is_none());
        assert!(value_of(&sample, Metric::NetIn).is_none());

        let (mem, mem_limit) = value_of(&sample, Metric::Mem).unwrap();
        assert_eq!(mem, 200.0);
        assert!(mem_limit.is_nan());
    }

    #[test]
    fn a_container_which_measured_nothing_gives_no_sample() {
        let mut json = running();
        json["state"] = "exited".into();
        json["cpu"]["usage"] = serde_json::Value::Null;
        json["mem"] = serde_json::json!({ "usage": null, "available": null, "limit": null });
        json["files"] = serde_json::json!({ "open": null, "limit": null });

        let sample = ServiceSample::of_container(
            "vm-1",
            &container(json),
            &NetUsageJsonMode::default(),
            Some(8),
            NOW,
        );

        assert!(sample.is_none());
    }

    #[test]
    fn a_machine_is_the_sum_of_its_containers_against_its_own_cores_and_ram() {
        let mut second = running();
        second["names"] = serde_json::json!(["/db"]);
        second["cpu"]["usage"] = 1.13.into();
        second["mem"]["usage"] = 300.into();
        second["files"]["open"] = 8.into();

        let mut totals = VmTotals::default();
        totals.add(&container(running()), &net(1.5, 0.25));
        totals.add(&container(second), &net(0.5, 0.25));

        let sample = totals
            .into_sample("vm-1", Some(8), Some(4000), NOW)
            .unwrap();

        assert_eq!(sample.vm, "vm-1");
        assert_eq!(sample.service, VM_SERIES);
        // The second the poll came at: the readings it sums are not of one second
        assert_eq!(sample.time as i64, NOW);

        assert_eq!(value_of(&sample, Metric::Cpu), Some((1.5, 8.0)));
        assert_eq!(value_of(&sample, Metric::Mem), Some((500.0, 4000.0)));

        // The limits of the containers do not add up to a limit of the machine
        let (files, files_limit) = value_of(&sample, Metric::Files).unwrap();
        assert_eq!(files, 50.0);
        assert!(files_limit.is_nan());
        assert_eq!(value_of(&sample, Metric::NetIn).unwrap().0, 2.0);
        assert_eq!(value_of(&sample, Metric::NetOut).unwrap().0, 0.5);
    }

    #[test]
    fn a_total_nothing_was_added_to_is_not_a_zero() {
        // The first poll: nothing has a rate yet. And a container which is not running
        let mut stopped = running();
        stopped["cpu"]["usage"] = serde_json::Value::Null;
        stopped["mem"] = serde_json::json!({ "usage": null, "available": null, "limit": null });
        stopped["files"] = serde_json::json!({ "open": null, "limit": null });

        let mut totals = VmTotals::default();
        totals.add(&container(running()), &NetUsageJsonMode::default());
        totals.add(&container(stopped.clone()), &NetUsageJsonMode::default());

        let sample = totals.into_sample("vm-1", None, None, NOW).unwrap();

        assert_eq!(value_of(&sample, Metric::Cpu).unwrap().0, 0.37);
        assert!(value_of(&sample, Metric::Cpu).unwrap().1.is_nan());
        assert!(value_of(&sample, Metric::NetIn).is_none());
        assert!(value_of(&sample, Metric::NetOut).is_none());

        // A machine none of whose containers measured anything leaves no record
        let mut totals = VmTotals::default();
        totals.add(&container(stopped), &NetUsageJsonMode::default());
        assert!(totals
            .into_sample("vm-1", Some(8), Some(4000), NOW)
            .is_none());
    }

    #[test]
    fn a_stamp_which_can_not_be_right_is_replaced_by_the_time_it_came() {
        assert_eq!(
            sample_time((NOW - 3) * 1000 + 999, NOW),
            Some((NOW - 3) as u32)
        );
        // A collector which does not stamp its readings
        assert_eq!(sample_time(0, NOW), Some(NOW as u32));
        // A host whose clock is days away
        assert_eq!(
            sample_time((NOW + 3 * 86_400) * 1000, NOW),
            Some(NOW as u32)
        );
        assert_eq!(
            sample_time((NOW - 3 * 86_400) * 1000, NOW),
            Some(NOW as u32)
        );
    }

    #[test]
    fn a_container_with_no_name_goes_by_the_start_of_its_id() {
        let mut json = running();
        json["names"] = serde_json::json!([]);

        assert_eq!(service_name(&container(json)), "0123456789ab");
        assert_eq!(service_name(&container(running())), "api");
    }
}
