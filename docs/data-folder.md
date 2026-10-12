# Data folder

`~/.docker-statistics-hub-data` is the one folder this service writes to. Mount
it as a volume (see the [README](../README.md#2-hub)) and everything the
service keeps survives the container; nothing else needs mounting for that.

Each kind of data is a **file of its own** in the folder — or a folder of its
own, when it is many files. Keeping something new later means one more entry
here — the mount stays as it is.

| Entry              | What it holds                                                                           |
| ------------------ | --------------------------------------------------------------------------------------- |
| `disk-titles.yaml` | Names given to host disks in the UI — see [Disk titles](disk-titles.md)                   |
| `metrics/`         | What the polls measured, service by service, in files by the day — see [Service metrics](service-metrics.md) |

**Storing something new** — the rule for whoever changes this service next: do
not add a mount, a second top-level folder, or a section to a file that is about
something else. Give the data a name of its own and take it from `DataFolder`
([data_folder.rs](../docker-statistics-hub/src/app/data_folder.rs)): `file(name)` the way `DiskTitles`
([disk_titles.rs](../docker-statistics-hub/src/app/disk_titles.rs)) does, or `folder(name)` for a kind of
data that is many files, the way `MetricsStorage`
([metrics_storage.rs](../docker-statistics-hub/src/metrics_storage/metrics_storage.rs)) does. Create the
store next to them in `AppCtx::new`, and add a row to the table above.

The folder is created on the first write. A file taken with `file(name)` is
replaced atomically — written beside itself and renamed — which is why the folder
is mounted and not a single file: a rename onto a file that is itself a mount
point is refused. Without the mount everything lives in the container's own
filesystem: it is gone when the container is recreated, and until then the
metrics grow there, by gigabytes.
