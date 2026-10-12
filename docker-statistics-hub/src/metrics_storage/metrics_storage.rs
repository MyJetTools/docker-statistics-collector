use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex,
    },
};

use crate::app::DataFolder;

use super::{DayFile, DayKey, Metric, Record, ServiceSample, SECONDS_PER_DAY};

/// The folder of the data folder the day files are kept in - theirs alone.
const FOLDER_NAME: &str = "metrics";

/// A day file nobody wrote into or read from for this long is closed
const IDLE_FILE_SECONDS: i64 = 10 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FileKey {
    env: String,
    vm: String,
    day: DayKey,
    metric: Metric,
}

struct OpenFile {
    file: Mutex<DayFile>,
    /// Unix seconds
    last_access: AtomicI64,
}

/// The measurements of the services, in files by the day:
/// `{root}/{env}/{vm}/{YYYYMMDD}.{metric}` - a file per metric, all of one format.
///
/// Everything here blocks on the disk: it is called from `spawn_blocking`.
pub struct MetricsStorage {
    root: PathBuf,
    files: Mutex<HashMap<FileKey, Arc<OpenFile>>>,
}

impl MetricsStorage {
    pub fn in_data_folder(folder: &DataFolder) -> Self {
        Self::new(folder.folder(FOLDER_NAME))
    }

    /// Nothing is touched on disk until the first write.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            files: Mutex::new(HashMap::new()),
        }
    }

    /// Writes what one poll of an env measured: every value into the file of its VM, its
    /// day and its metric. Returns what went wrong.
    pub fn write(&self, env: &str, samples: &[ServiceSample], now: i64) -> Vec<String> {
        let mut by_file: BTreeMap<(&str, DayKey, Metric), Vec<(&str, Record)>> = BTreeMap::new();

        for sample in samples {
            let day = DayKey::of(sample.time as i64);

            for value in &sample.values {
                by_file
                    .entry((sample.vm.as_str(), day, value.metric))
                    .or_default()
                    .push((
                        sample.service.as_str(),
                        Record {
                            time: sample.time,
                            limit: value.limit,
                            value: value.value,
                        },
                    ));
            }
        }

        let mut messages = Vec::new();

        for ((vm, day, metric), records) in by_file {
            let name = file_name(day, metric);

            let handle = match self.get_file(env, vm, day, metric, true, now) {
                Ok(Some(handle)) => handle,
                Ok(None) => continue,
                Err(err) => {
                    messages.push(format!("{}/{}/{} does not open: {}", env, vm, name, err));
                    continue;
                }
            };

            let mut file = handle.file.lock().unwrap();

            for (service, record) in records {
                if let Err(err) = file.append(service, &record) {
                    // The rest of the file would say the same
                    messages.push(format!(
                        "{}/{}/{}: {} is not written: {}",
                        env, vm, name, service, err
                    ));
                    break;
                }
            }
        }

        messages
    }

    /// The records of the metric of the service in `from .. to`, unix seconds, in time
    /// order - read from the file of every day the interval touches.
    pub fn read(
        &self,
        env: &str,
        vm: &str,
        service: &str,
        metric: Metric,
        from: i64,
        to: i64,
        now: i64,
    ) -> Result<Vec<Record>, String> {
        let mut result = Vec::new();

        let from = from.clamp(0, u32::MAX as i64);
        let to = to.clamp(0, u32::MAX as i64);

        for day in DayKey::range(from, to) {
            let handle = self
                .get_file(env, vm, day, metric, false, now)
                .map_err(|err| {
                    format!(
                        "{}/{}/{} does not open: {}",
                        env,
                        vm,
                        file_name(day, metric),
                        err
                    )
                })?;

            let Some(handle) = handle else {
                continue;
            };

            handle
                .file
                .lock()
                .unwrap()
                .read(service, from as u32, to as u32, &mut result)
                .map_err(|err| {
                    format!(
                        "{}/{}/{} does not read: {}",
                        env,
                        vm,
                        file_name(day, metric),
                        err
                    )
                })?;
        }

        // A file has them in the order they came - which a clock set back makes another
        result.sort_by_key(|record| record.time);

        Ok(result)
    }

    /// The day file, opened when it is not yet. Without `create` a day with no file is
    /// `None` - a read does not make one.
    fn get_file(
        &self,
        env: &str,
        vm: &str,
        day: DayKey,
        metric: Metric,
        create: bool,
        now: i64,
    ) -> std::io::Result<Option<Arc<OpenFile>>> {
        let key = FileKey {
            env: env.to_string(),
            vm: vm.to_string(),
            day,
            metric,
        };

        let mut files = self.files.lock().unwrap();

        if let Some(file) = files.get(&key) {
            file.last_access.store(now, Ordering::Relaxed);
            return Ok(Some(file.clone()));
        }

        let dir = self.dir(env, vm);

        if create {
            std::fs::create_dir_all(&dir)?;
        }

        let Some(file) = DayFile::open(&dir.join(file_name(day, metric)), create)? else {
            return Ok(None);
        };

        let file = Arc::new(OpenFile {
            file: Mutex::new(file),
            last_access: AtomicI64::new(now),
        });

        files.insert(key, file.clone());

        Ok(Some(file))
    }

    fn dir(&self, env: &str, vm: &str) -> PathBuf {
        self.root.join(path_part(env)).join(path_part(vm))
    }

    /// Deletes the files of the days older than `days_to_keep` days, and the folders
    /// they leave empty; closes the files nobody uses. Returns what was done.
    pub fn gc(&self, now: i64, days_to_keep: u32) -> Vec<String> {
        let first_kept = DayKey::of(now - days_to_keep as i64 * SECONDS_PER_DAY);

        self.files.lock().unwrap().retain(|key, file| {
            key.day >= first_kept
                && now - file.last_access.load(Ordering::Relaxed) < IDLE_FILE_SECONDS
        });

        let mut messages = Vec::new();

        for env_dir in sub_dirs(&self.root) {
            for vm_dir in sub_dirs(&env_dir) {
                for file in files_of(&vm_dir) {
                    let expired = file
                        .file_name()
                        .and_then(|name| name.to_str())
                        .and_then(day_of_file)
                        .is_some_and(|day| day < first_kept);

                    if !expired {
                        continue;
                    }

                    match std::fs::remove_file(&file) {
                        Ok(()) => messages.push(format!("GC: {} is deleted", file.display())),
                        Err(err) => {
                            messages.push(format!("GC: {} is not deleted: {}", file.display(), err))
                        }
                    }
                }

                // Only an empty folder goes: a VM which is gone leaves nothing behind
                // once its last day is deleted
                let _ = std::fs::remove_dir(&vm_dir);
            }

            let _ = std::fs::remove_dir(&env_dir);
        }

        messages
    }

    #[cfg(test)]
    fn open_files(&self) -> usize {
        self.files.lock().unwrap().len()
    }
}

/// `20261012.cpu`
fn file_name(day: DayKey, metric: Metric) -> String {
    format!("{}.{}", day, metric.as_str())
}

/// The day of a metric file by its name. Anything else in the folder is nobody's to delete.
fn day_of_file(name: &str) -> Option<DayKey> {
    let (day, metric) = name.split_once('.')?;
    Metric::parse(metric)?;
    DayKey::parse(day)
}

/// The name of an env or of a VM as a folder: what is not a letter, a digit, `-`, `_` or a
/// `.` inside the name is written as `%XX`, so no name leads out of its folder and no two
/// names share one. A name of nothing is `%`.
fn path_part(name: &str) -> String {
    if name.is_empty() {
        return "%".to_string();
    }

    let mut result = String::with_capacity(name.len());

    for (index, byte) in name.bytes().enumerate() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' => result.push(byte as char),
            b'.' if index > 0 => result.push('.'),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }

    result
}

fn sub_dirs(path: &Path) -> Vec<PathBuf> {
    entries_of(path, true)
}

fn files_of(path: &Path) -> Vec<PathBuf> {
    entries_of(path, false)
}

fn entries_of(path: &Path, dirs: bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|tp| tp.is_dir() == dirs))
        .map(|entry| entry.path())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::MetricValue;
    use super::*;

    /// 2026-10-12T00:00:00Z
    const OCT_12: i64 = 1_791_763_200;

    /// A storage folder of its own per test, under the system temp dir.
    fn temp_folder(test: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "docker-statistics-hub-metrics-{}-{}",
            test,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    fn sample(vm: &str, service: &str, time: i64, cpu: f64) -> ServiceSample {
        ServiceSample {
            vm: vm.to_string(),
            service: service.to_string(),
            time: time as u32,
            values: vec![
                MetricValue {
                    metric: Metric::Cpu,
                    value: cpu,
                    limit: 8.0,
                },
                MetricValue {
                    metric: Metric::NetIn,
                    value: cpu * 10.0,
                    limit: f64::NAN,
                },
            ],
        }
    }

    fn values(records: &[Record]) -> Vec<(i64, f64)> {
        records
            .iter()
            .map(|record| (record.time as i64, record.value))
            .collect()
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_value_lands_in_the_file_of_its_vm_its_day_and_its_metric() {
        let path = temp_folder("layout");
        let storage = MetricsStorage::new(&path);
        let now = OCT_12 + 100;

        let messages = storage.write(
            "prod",
            &[
                sample("vm-1", "api", now, 0.5),
                sample("vm-2", "api", now, 0.7),
            ],
            now,
        );
        assert!(messages.is_empty(), "{:?}", messages);

        assert_eq!(
            file_names(&path.join("prod").join("vm-1")),
            ["20261012.cpu", "20261012.net-in"]
        );

        let read = |vm, metric| {
            storage
                .read("prod", vm, "api", metric, OCT_12, OCT_12 + SECONDS_PER_DAY, now)
                .unwrap()
        };

        assert_eq!(values(&read("vm-1", Metric::Cpu)), [(now, 0.5)]);
        assert_eq!(values(&read("vm-2", Metric::Cpu)), [(now, 0.7)]);
        assert_eq!(values(&read("vm-1", Metric::NetIn)), [(now, 5.0)]);
        // Nothing was measured, so there is no file - and a read does not make one
        assert!(read("vm-1", Metric::Mem).is_empty());
        assert_eq!(file_names(&path.join("prod").join("vm-1")).len(), 2);

        // The same VM name in another env is another VM
        assert!(storage
            .read("stage", "vm-1", "api", Metric::Cpu, OCT_12, now + 1, now)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn an_interval_is_read_across_midnight_in_time_order() {
        let path = temp_folder("midnight");
        let storage = MetricsStorage::new(&path);

        let before = OCT_12 - 3;
        let after = OCT_12 + 3;

        storage.write("prod", &[sample("vm-1", "api", before, 1.0)], after);
        storage.write("prod", &[sample("vm-1", "api", after + 3, 3.0)], after + 3);
        // A host whose clock was set back: an earlier second comes after a later one
        storage.write("prod", &[sample("vm-1", "api", after, 2.0)], after + 3);

        assert_eq!(
            file_names(&path.join("prod").join("vm-1")),
            [
                "20261011.cpu",
                "20261011.net-in",
                "20261012.cpu",
                "20261012.net-in"
            ]
        );

        let read = |from, to| {
            values(
                &storage
                    .read("prod", "vm-1", "api", Metric::Cpu, from, to, after + 3)
                    .unwrap(),
            )
        };

        assert_eq!(
            read(OCT_12 - 10, OCT_12 + 10),
            [(before, 1.0), (after, 2.0), (after + 3, 3.0)]
        );
        // `to` is not in the interval
        assert_eq!(read(OCT_12 - 10, after), [(before, 1.0)]);
        assert_eq!(read(after, after + 3), [(after, 2.0)]);
        assert!(read(after, after).is_empty());
    }

    #[test]
    fn a_restart_goes_on_writing_into_the_same_files() {
        let path = temp_folder("restart");
        let now = OCT_12 + 100;

        MetricsStorage::new(&path).write("prod", &[sample("vm-1", "api", now, 0.5)], now);

        // A second instance over the same folder is what a restart is.
        let storage = MetricsStorage::new(&path);
        storage.write("prod", &[sample("vm-1", "api", now + 3, 0.6)], now + 3);

        let records = storage
            .read("prod", "vm-1", "api", Metric::Cpu, OCT_12, now + 10, now + 3)
            .unwrap();

        assert_eq!(values(&records), [(now, 0.5), (now + 3, 0.6)]);
    }

    #[test]
    fn gc_deletes_the_days_past_their_time_and_the_folders_they_leave_empty() {
        let path = temp_folder("gc");
        let storage = MetricsStorage::new(&path);

        let now = OCT_12 + 100;
        let week_ago = now - 7 * SECONDS_PER_DAY;
        let long_ago = now - 8 * SECONDS_PER_DAY;

        storage.write("prod", &[sample("vm-1", "api", now, 1.0)], now);
        storage.write("prod", &[sample("vm-1", "api", week_ago, 2.0)], now);
        storage.write("prod", &[sample("vm-1", "api", long_ago, 3.0)], now);
        // A VM which is gone: all it has is past its time
        storage.write("old", &[sample("vm-9", "api", long_ago, 4.0)], now);

        // Something which is not a metric file is nobody's to delete
        std::fs::write(path.join("prod").join("vm-1").join("20260101.txt"), "x").unwrap();

        let messages = storage.gc(now, 7);

        assert_eq!(messages.len(), 4, "{:?}", messages);
        assert_eq!(
            file_names(&path.join("prod").join("vm-1")),
            [
                "20260101.txt",
                "20261005.cpu",
                "20261005.net-in",
                "20261012.cpu",
                "20261012.net-in"
            ]
        );
        assert_eq!(file_names(&path), ["prod"]);

        // And what is deleted is not read from a file left open
        assert!(storage
            .read("prod", "vm-1", "api", Metric::Cpu, long_ago - 10, long_ago + 10, now)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn gc_closes_the_files_nobody_uses() {
        let path = temp_folder("idle");
        let storage = MetricsStorage::new(&path);
        let now = OCT_12 + 100;

        storage.write("prod", &[sample("vm-1", "api", now, 1.0)], now);
        assert_eq!(storage.open_files(), 2);

        storage.gc(now + IDLE_FILE_SECONDS - 1, 7);
        assert_eq!(storage.open_files(), 2);

        storage.gc(now + IDLE_FILE_SECONDS, 7);
        assert_eq!(storage.open_files(), 0);

        // A closed file is opened again by whoever needs it
        let later = now + IDLE_FILE_SECONDS;
        storage.write("prod", &[sample("vm-1", "api", later, 2.0)], later);

        let records = storage
            .read("prod", "vm-1", "api", Metric::Cpu, OCT_12, later + 1, later)
            .unwrap();
        assert_eq!(values(&records), [(now, 1.0), (later, 2.0)]);
    }

    #[test]
    fn a_name_never_leads_out_of_its_folder() {
        assert_eq!(path_part("prod"), "prod");
        assert_eq!(path_part("vm-01.eu_west"), "vm-01.eu_west");
        assert_eq!(path_part(".."), "%2E.");
        assert_eq!(path_part("."), "%2E");
        assert_eq!(path_part("a/b"), "a%2Fb");
        assert_eq!(path_part("a b"), "a%20b");
        assert_eq!(path_part(""), "%");
        // Two names never share a folder
        assert_ne!(path_part("a%2Fb"), path_part("a/b"));
    }

    #[test]
    fn only_a_metric_file_has_a_day() {
        assert_eq!(day_of_file("20261012.cpu"), DayKey::parse("20261012"));
        assert_eq!(day_of_file("20261012.net-in"), DayKey::parse("20261012"));
        assert_eq!(day_of_file("20261012.txt"), None);
        assert_eq!(day_of_file("20261012"), None);
        assert_eq!(day_of_file("notes.cpu"), None);
        assert_eq!(day_of_file("20261012.cpu.tmp"), None);
    }
}
