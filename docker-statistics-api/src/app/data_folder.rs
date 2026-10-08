use std::path::PathBuf;

/// The one folder this service writes to — and so the one thing that has to be
/// mounted for its data to outlive the container. Each kind of data it keeps is
/// a file of its own in here: keeping something new means adding a file to this
/// folder, not another mount and not another section in a file that is about
/// something else.
const DATA_FOLDER: &str = "~/.docker-statistics-api-data";

pub struct DataFolder {
    path: PathBuf,
}

impl DataFolder {
    pub fn new() -> Self {
        Self::at(rust_extensions::file_utils::format_path(DATA_FOLDER).as_str())
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// One file of the folder, by name. Nothing is touched on disk until the
    /// file is read or written — the folder itself is created by the first write.
    pub fn file(&self, name: &str) -> DataFile {
        DataFile {
            path: self.path.join(name),
        }
    }
}

/// One file of the [`DataFolder`], read whole and replaced whole.
pub struct DataFile {
    path: PathBuf,
}

impl DataFile {
    /// The content, or `None` when the file is not there yet — the ordinary
    /// state before anything was saved. Blocking: it is for loading at startup.
    pub fn read(&self) -> Result<Option<String>, String> {
        match std::fs::read_to_string(&self.path) {
            Ok(content) => Ok(Some(content)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(format!("cannot read {}: {}", self, err)),
        }
    }

    /// Replace the file with `content`. It is written beside the file and
    /// renamed over it, so a crash mid-write leaves the previous content rather
    /// than half of the new one. That rename is why the folder is what gets
    /// mounted: onto a file that is itself a mount point it is refused.
    pub async fn write(&self, content: &str) -> Result<(), String> {
        if let Some(folder) = self.path.parent() {
            tokio::fs::create_dir_all(folder)
                .await
                .map_err(|err| format!("cannot create {}: {}", folder.display(), err))?;
        }

        let mut tmp_path = self.path.clone().into_os_string();
        tmp_path.push(".tmp");
        let tmp_path = PathBuf::from(tmp_path);

        tokio::fs::write(&tmp_path, content)
            .await
            .map_err(|err| format!("cannot write {}: {}", tmp_path.display(), err))?;
        tokio::fs::rename(&tmp_path, &self.path)
            .await
            .map_err(|err| format!("cannot replace {}: {}", self, err))
    }
}

impl std::fmt::Display for DataFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own per test, under the system temp dir, not created yet.
    fn temp_folder(test: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "docker-statistics-api-data-folder-{}-{}",
            test,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[tokio::test]
    async fn a_file_that_was_never_written_reads_as_none() {
        let folder = DataFolder::at(temp_folder("missing"));
        assert_eq!(folder.file("anything.yaml").read(), Ok(None));
    }

    #[tokio::test]
    async fn the_first_write_creates_the_folder_and_leaves_nothing_but_the_file() {
        let path = temp_folder("first-write");
        let folder = DataFolder::at(&path);

        folder.file("one.yaml").write("a: 1\n").await.unwrap();

        assert_eq!(
            folder.file("one.yaml").read(),
            Ok(Some("a: 1\n".to_string()))
        );
        let names: Vec<_> = std::fs::read_dir(&path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec!["one.yaml"]);
    }

    #[tokio::test]
    async fn each_kind_of_data_is_a_file_of_its_own() {
        let folder = DataFolder::at(temp_folder("two-files"));

        folder.file("one.yaml").write("first").await.unwrap();
        folder.file("two.yaml").write("second").await.unwrap();
        folder.file("one.yaml").write("first, again").await.unwrap();

        assert_eq!(
            folder.file("one.yaml").read(),
            Ok(Some("first, again".to_string()))
        );
        assert_eq!(
            folder.file("two.yaml").read(),
            Ok(Some("second".to_string()))
        );
    }
}
