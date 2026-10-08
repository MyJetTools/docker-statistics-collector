use std::collections::BTreeMap;

use tokio::sync::RwLock;

use crate::models::VmModel;

use super::{DataFile, DataFolder};

/// The file of the data folder the titles are kept in — theirs alone.
const FILE_NAME: &str = "disk-titles.yaml";

/// Longest title accepted, in characters — it has to fit a line of the VM rail.
pub const MAX_DISK_TITLE_LEN: usize = 48;

const FILE_HEADER: &str =
    "# Disk titles set from the docker-statistics UI: env -> vm -> mount point -> title.\n";

/// `env -> vm -> disk -> title`. A disk is named by its mount point: that is
/// what stays put when a volume is re-attached and its `/dev/sdX` moves.
type TitlesByEnv = BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>;

/// Operator-given names for host disks, kept in a file of the data folder.
/// Everything else this service holds is a cache of what the collectors report
/// and can be lost on a restart; a title somebody typed can not.
pub struct DiskTitles {
    file: DataFile,
    data: RwLock<TitlesByEnv>,
    /// Set when the file exists but could not be read as titles. Saving is then
    /// refused: writing would replace a file somebody may have edited by hand
    /// with whatever was parsed out of it, which is nothing.
    load_error: Option<String>,
}

impl DiskTitles {
    pub fn load(folder: &DataFolder) -> Self {
        let file = folder.file(FILE_NAME);

        let (data, load_error) = match file.read() {
            Ok(Some(content)) => match parse(content.as_str()) {
                Ok(data) => (data, None),
                Err(err) => {
                    let err = format!("{} is not a valid titles file: {}", file, err);
                    println!(
                        "DiskTitles: {}. Titles are off until it is fixed or removed.",
                        err
                    );
                    (TitlesByEnv::new(), Some(err))
                }
            },
            // No file yet is the ordinary state before the first title is set.
            Ok(None) => (TitlesByEnv::new(), None),
            Err(err) => {
                println!("DiskTitles: {}. Titles are off until that is fixed.", err);
                (TitlesByEnv::new(), Some(err))
            }
        };

        Self {
            file,
            data: RwLock::new(data),
            load_error,
        }
    }

    /// Stamp the stored titles onto the disks of `vms`. Every disk is written,
    /// titled or not, so a disk whose title was removed loses it on the next poll.
    pub async fn apply(&self, env: &str, vms: &mut BTreeMap<String, VmModel>) {
        let data = self.data.read().await;
        let by_vm = data.get(env);

        for (vm_name, vm) in vms.iter_mut() {
            let titles = by_vm.and_then(|by_vm| by_vm.get(vm_name));

            for disk in vm.host_disks.iter_mut().flatten() {
                disk.title = titles
                    .and_then(|titles| titles.get(disk.mount_point.as_str()))
                    .cloned();
            }
        }
    }

    /// Set the title of one disk, or remove it with `None`. The change is on disk
    /// before it is in memory: a title the UI shows as saved has to survive a
    /// restart, so a write that fails fails the whole call.
    pub async fn set(
        &self,
        env: &str,
        vm: &str,
        disk: &str,
        title: Option<String>,
    ) -> Result<(), String> {
        if let Some(err) = self.load_error.as_ref() {
            return Err(err.clone());
        }

        let mut data = self.data.write().await;

        let mut next = data.clone();
        match title {
            Some(title) => {
                next.entry(env.to_string())
                    .or_default()
                    .entry(vm.to_string())
                    .or_default()
                    .insert(disk.to_string(), title);
            }
            None => remove(&mut next, env, vm, disk),
        }

        if next == *data {
            return Ok(());
        }

        self.save(&next).await?;
        *data = next;
        Ok(())
    }

    async fn save(&self, data: &TitlesByEnv) -> Result<(), String> {
        let content = serde_yaml::to_string(data)
            .map_err(|err| format!("cannot serialize disk titles: {}", err))?;

        self.file
            .write(format!("{}{}", FILE_HEADER, content).as_str())
            .await
    }
}

fn parse(content: &str) -> Result<TitlesByEnv, serde_yaml::Error> {
    // A file holding nothing but the header comment parses as `null`.
    Ok(serde_yaml::from_str::<Option<TitlesByEnv>>(content)?.unwrap_or_default())
}

/// Drop one title, and with it every level it leaves empty — the file lists
/// titles, not every VM that ever had one.
fn remove(data: &mut TitlesByEnv, env: &str, vm: &str, disk: &str) {
    let Some(by_vm) = data.get_mut(env) else {
        return;
    };
    let Some(titles) = by_vm.get_mut(vm) else {
        return;
    };

    titles.remove(disk);
    if titles.is_empty() {
        by_vm.remove(vm);
    }
    if by_vm.is_empty() {
        data.remove(env);
    }
}

/// What a typed title becomes before it is stored: trimmed, and `None` when
/// nothing is left — an empty title is the request to remove it.
pub fn normalize_disk_title(raw: Option<&str>) -> Result<Option<String>, String> {
    let title = raw.unwrap_or_default().trim();

    if title.is_empty() {
        return Ok(None);
    }
    if title.chars().any(|c| c.is_control()) {
        return Err("a disk title is a single line of text".to_string());
    }
    if title.chars().count() > MAX_DISK_TITLE_LEN {
        return Err(format!(
            "a disk title is at most {} characters",
            MAX_DISK_TITLE_LEN
        ));
    }

    Ok(Some(title.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DiskModel;

    /// A data folder of its own per test, under the system temp dir.
    fn temp_folder(test: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "docker-statistics-api-disk-titles-{}-{}",
            test,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    fn vm_with_disks(mount_points: &[&str]) -> VmModel {
        VmModel {
            api_url: String::new(),
            cpu: 0.0,
            mem: 0,
            mem_limit: 0,
            containers_amount: 0,
            open_files: 0,
            net_in_mbps: 0.0,
            net_out_mbps: 0.0,
            host_mem_total: None,
            host_mem_available: None,
            host_mem_used: None,
            host_cpu_count: None,
            host_disks: Some(
                mount_points
                    .iter()
                    .map(|mount_point| DiskModel {
                        device: format!("/dev{}", mount_point),
                        mount_point: mount_point.to_string(),
                        fs_type: "ext4".to_string(),
                        total: 100,
                        used: 50,
                        available: 50,
                        title: None,
                    })
                    .collect(),
            ),
        }
    }

    fn titles_of(vms: &BTreeMap<String, VmModel>, vm: &str) -> Vec<Option<String>> {
        vms[vm]
            .host_disks
            .iter()
            .flatten()
            .map(|disk| disk.title.clone())
            .collect()
    }

    #[tokio::test]
    async fn a_title_survives_a_restart_and_lands_on_its_own_disk_only() {
        let path = temp_folder("restart");

        let titles = DiskTitles::load(&DataFolder::at(&path));
        titles
            .set("prod", "db-01", "/data", Some("Postgres".to_string()))
            .await
            .unwrap();
        assert!(path.join(FILE_NAME).is_file());

        // A second instance over the same folder is what a restart is.
        let titles = DiskTitles::load(&DataFolder::at(&path));
        let mut vms = BTreeMap::new();
        vms.insert("db-01".to_string(), vm_with_disks(&["/", "/data"]));
        vms.insert("db-02".to_string(), vm_with_disks(&["/data"]));

        titles.apply("prod", &mut vms).await;
        assert_eq!(
            titles_of(&vms, "db-01"),
            vec![None, Some("Postgres".to_string())]
        );
        assert_eq!(titles_of(&vms, "db-02"), vec![None]);

        // The same VM name in another env is another VM.
        titles.apply("stage", &mut vms).await;
        assert_eq!(titles_of(&vms, "db-01"), vec![None, None]);
    }

    #[tokio::test]
    async fn removing_the_last_title_leaves_no_empty_levels_behind() {
        let path = temp_folder("remove");

        let titles = DiskTitles::load(&DataFolder::at(&path));
        titles
            .set("prod", "db-01", "/", Some("System".to_string()))
            .await
            .unwrap();
        titles.set("prod", "db-01", "/", None).await.unwrap();

        let content = std::fs::read_to_string(path.join(FILE_NAME)).unwrap();
        assert_eq!(parse(content.as_str()).unwrap(), TitlesByEnv::new());
    }

    #[tokio::test]
    async fn a_file_that_cannot_be_parsed_is_never_overwritten() {
        let path = temp_folder("broken");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(FILE_NAME), "prod: [not, a, map]\n").unwrap();

        let titles = DiskTitles::load(&DataFolder::at(&path));
        let result = titles
            .set("prod", "db-01", "/", Some("System".to_string()))
            .await;

        assert!(result.is_err());
        assert_eq!(
            std::fs::read_to_string(path.join(FILE_NAME)).unwrap(),
            "prod: [not, a, map]\n"
        );
    }

    #[test]
    fn a_typed_title_is_trimmed_and_an_empty_one_means_remove() {
        assert_eq!(
            normalize_disk_title(Some("  Postgres data ")),
            Ok(Some("Postgres data".to_string()))
        );
        assert_eq!(normalize_disk_title(Some("   ")), Ok(None));
        assert_eq!(normalize_disk_title(None), Ok(None));
    }

    #[test]
    fn a_title_that_would_break_the_rail_is_refused() {
        assert!(normalize_disk_title(Some("two\nlines")).is_err());
        assert!(normalize_disk_title(Some("x".repeat(MAX_DISK_TITLE_LEN + 1).as_str())).is_err());
        assert!(normalize_disk_title(Some("x".repeat(MAX_DISK_TITLE_LEN).as_str())).is_ok());
    }
}
