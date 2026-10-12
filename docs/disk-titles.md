# Disk titles

A host disk can be given a name in the UI — click its icon in the VM rail. The
title is shown in place of the mount point, there and on the Host disks board.
Everything else this service holds is a cache of what the collectors report; a
title somebody typed is not, so it is kept in the [data folder](data-folder.md).

`disk-titles.yaml`:

```yaml
# env -> vm -> mount point -> title
prod:
  vm-prod-db-01:
    /var/lib/postgresql: Postgres data
```

- A disk is identified by its **mount point**, not its device: `/dev/sdX` can
  move when a volume is re-attached, the mount point does not.
- `POST /api/disk-title` sets a title; an empty (or absent) `title` removes it.
  Titles are trimmed, single-line and at most 48 characters. The caller needs
  access to the env, same as for every other endpoint.
- The file is rewritten on every change and read once, at startup — after
  editing it by hand, restart the service. A file that cannot be parsed is left
  alone: titles stay off and saving is refused until it is fixed or removed.
