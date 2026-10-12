use super::{Metric, MetricsStorage, Record, SECONDS_PER_DAY};

/// What the records of one stretch of a chart come to
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartPoint {
    /// Unix second the stretch starts at
    pub time: u32,
    pub avg: f64,
    pub min: f64,
    pub max: f64,
    /// The limit of the last record of the stretch; NaN when it has none
    pub limit: f64,
}

/// A chart of a metric: its points in time order, each standing for `step` seconds. A
/// stretch nothing was measured in has no point - a service which was down is a gap,
/// not a line through zero.
#[derive(Debug, Clone, PartialEq)]
pub struct Chart {
    pub step: i64,
    pub points: Vec<ChartPoint>,
}

#[derive(Clone, Copy)]
struct Bucket {
    count: u32,
    sum: f64,
    min: f64,
    max: f64,
    last_time: u32,
    limit: f64,
}

/// An interval cut into stretches of one length - no more of them than a chart has room
/// for - and the records folded into them as they come. A week of a metric is two
/// hundred thousand records and a chart is a few hundred pixels wide: the average of a
/// stretch is what the line goes through, its minimum and maximum are what keeps a spike
/// from being averaged away.
pub struct ChartBuckets {
    from: i64,
    to: i64,
    step: i64,
    buckets: Vec<Option<Bucket>>,
}

impl ChartBuckets {
    pub fn new(from: i64, to: i64, max_points: usize) -> Self {
        let span = (to - from).max(1);
        let max_points = max_points.max(1) as i64;

        // The shortest stretch the interval is covered by with no more than `max_points`
        let step = (span + max_points - 1) / max_points;
        let count = (span + step - 1) / step;

        Self {
            from,
            to,
            step,
            buckets: vec![None; count as usize],
        }
    }

    pub fn add(&mut self, record: &Record) {
        let time = record.time as i64;

        if time < self.from || time >= self.to || !record.value.is_finite() {
            return;
        }

        let index = ((time - self.from) / self.step) as usize;

        let Some(slot) = self.buckets.get_mut(index) else {
            return;
        };

        match slot {
            Some(bucket) => {
                bucket.count += 1;
                bucket.sum += record.value;
                bucket.min = bucket.min.min(record.value);
                bucket.max = bucket.max.max(record.value);

                // The records of a day file come in the order they were written, which
                // a clock set back makes another than the order of time
                if record.time >= bucket.last_time {
                    bucket.last_time = record.time;
                    bucket.limit = record.limit;
                }
            }
            None => {
                *slot = Some(Bucket {
                    count: 1,
                    sum: record.value,
                    min: record.value,
                    max: record.value,
                    last_time: record.time,
                    limit: record.limit,
                });
            }
        }
    }

    pub fn into_chart(self) -> Chart {
        let from = self.from;
        let step = self.step;

        let points = self
            .buckets
            .into_iter()
            .enumerate()
            .filter_map(|(index, bucket)| {
                let bucket = bucket?;

                Some(ChartPoint {
                    time: (from + index as i64 * step) as u32,
                    avg: bucket.sum / bucket.count as f64,
                    min: bucket.min,
                    max: bucket.max,
                    limit: bucket.limit,
                })
            })
            .collect();

        Chart { step, points }
    }
}

impl MetricsStorage {
    /// The metric of the service over `from .. to`, unix seconds, as no more than
    /// `max_points` points. It is read a day at a time: what is in memory is the records
    /// of one day, however long the interval is.
    #[allow(clippy::too_many_arguments)]
    pub fn read_chart(
        &self,
        env: &str,
        vm: &str,
        service: &str,
        metric: Metric,
        from: i64,
        to: i64,
        max_points: usize,
        now: i64,
    ) -> Result<Chart, String> {
        let from = from.clamp(0, u32::MAX as i64);
        let to = to.clamp(0, u32::MAX as i64);

        let mut buckets = ChartBuckets::new(from, to, max_points);

        let mut day_from = from;

        while day_from < to {
            let day_to = (day_from + SECONDS_PER_DAY).min(to);

            for record in self.read(env, vm, service, metric, day_from, day_to, now)? {
                buckets.add(&record);
            }

            day_from = day_to;
        }

        Ok(buckets.into_chart())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{MetricValue, ServiceSample};
    use super::*;

    fn record(time: u32, value: f64) -> Record {
        Record {
            time,
            limit: f64::NAN,
            value,
        }
    }

    fn chart_of(from: i64, to: i64, max_points: usize, records: &[Record]) -> Chart {
        let mut buckets = ChartBuckets::new(from, to, max_points);

        for record in records {
            buckets.add(record);
        }

        buckets.into_chart()
    }

    #[test]
    fn a_stretch_is_the_average_the_minimum_and_the_maximum_of_its_records() {
        let chart = chart_of(
            1000,
            1060,
            2,
            &[
                record(1000, 1.0),
                record(1010, 3.0),
                record(1029, 8.0),
                record(1030, 10.0),
            ],
        );

        assert_eq!(chart.step, 30);
        assert_eq!(chart.points.len(), 2);

        assert_eq!(chart.points[0].time, 1000);
        assert_eq!(chart.points[0].avg, 4.0);
        assert_eq!(chart.points[0].min, 1.0);
        assert_eq!(chart.points[0].max, 8.0);

        assert_eq!(chart.points[1].time, 1030);
        assert_eq!(chart.points[1].avg, 10.0);
    }

    #[test]
    fn a_stretch_nothing_was_measured_in_has_no_point() {
        let chart = chart_of(0, 100, 10, &[record(5, 1.0), record(95, 2.0)]);

        let times: Vec<u32> = chart.points.iter().map(|point| point.time).collect();
        assert_eq!(times, vec![0, 90]);
    }

    #[test]
    fn there_are_never_more_points_than_asked_for() {
        // 100 seconds do not divide into 7 stretches: they are made longer, not more
        let records: Vec<Record> = (0..100).map(|time| record(1000 + time, 1.0)).collect();
        let chart = chart_of(1000, 1100, 7, &records);

        assert_eq!(chart.step, 15);
        assert_eq!(chart.points.len(), 7);

        // An interval shorter than the room for it is a point a second, no finer
        let chart = chart_of(1000, 1010, 500, &records);
        assert_eq!(chart.step, 1);
        assert_eq!(chart.points.len(), 10);
    }

    #[test]
    fn what_is_outside_of_the_interval_or_is_not_a_number_is_left_out() {
        let chart = chart_of(
            100,
            200,
            1,
            &[
                record(99, 50.0),
                record(100, 1.0),
                record(150, f64::NAN),
                record(199, 3.0),
                record(200, 50.0),
            ],
        );

        assert_eq!(chart.points.len(), 1);
        assert_eq!(chart.points[0].avg, 2.0);
        assert_eq!(chart.points[0].max, 3.0);
    }

    #[test]
    fn the_limit_of_a_stretch_is_the_one_of_its_latest_record() {
        let mut buckets = ChartBuckets::new(0, 100, 1);

        for (time, limit) in [(50, 4.0), (10, 2.0), (90, 8.0), (70, 6.0)] {
            buckets.add(&Record {
                time,
                limit,
                value: 1.0,
            });
        }

        assert_eq!(buckets.into_chart().points[0].limit, 8.0);
    }

    /// 2026-10-12T00:00:00Z
    const OCT_12: i64 = 1_791_763_200;

    #[test]
    fn a_chart_of_several_days_is_read_from_the_file_of_each() {
        let path = std::env::temp_dir().join(format!(
            "docker-statistics-hub-metrics-chart-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);

        let storage = MetricsStorage::new(&path);

        // An hour before midnight, and an hour after it: two day files
        let samples: Vec<ServiceSample> = [(-3600, 2.0), (-1800, 4.0), (1800, 6.0), (3600, 8.0)]
            .into_iter()
            .map(|(offset, value)| ServiceSample {
                vm: "vm".to_string(),
                service: "api".to_string(),
                time: (OCT_12 + offset) as u32,
                values: vec![MetricValue {
                    metric: Metric::Cpu,
                    value,
                    limit: 16.0,
                }],
            })
            .collect();

        assert!(storage.write("prod", &samples, OCT_12).is_empty());

        let chart = storage
            .read_chart(
                "prod",
                "vm",
                "api",
                Metric::Cpu,
                OCT_12 - 7200,
                OCT_12 + 7200,
                2,
                OCT_12,
            )
            .unwrap();

        assert_eq!(chart.step, 7200);
        assert_eq!(chart.points.len(), 2);
        assert_eq!(chart.points[0].avg, 3.0);
        assert_eq!(chart.points[1].avg, 7.0);
        assert_eq!(chart.points[1].limit, 16.0);

        // Another service of the same VM has no part in it
        let other = storage
            .read_chart(
                "prod",
                "vm",
                "db",
                Metric::Cpu,
                OCT_12 - 7200,
                OCT_12 + 7200,
                2,
                OCT_12,
            )
            .unwrap();
        assert!(other.points.is_empty());

        let _ = std::fs::remove_dir_all(&path);
    }
}
