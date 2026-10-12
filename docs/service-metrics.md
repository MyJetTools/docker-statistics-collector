# Service metrics

Every poll is also written down: what each service measured goes to disk, so its
history outlives both the in-memory cache and a restart. The cache keeps the last
minutes for the live graphs; this keeps days.

| Metric    | Value                         | Limit kept next to it           |
| --------- | ----------------------------- | ------------------------------- |
| `cpu`     | cpu in use, as the collector reports it | cores of the host     |
| `mem`     | bytes of memory in use        | memory limit of the container   |
| `files`   | open file descriptors         | its `nofile` limit              |
| `net-in`  | inbound throughput, MB/s      | —                               |
| `net-out` | outbound throughput, MB/s     | —                               |

- A **service** is a container by its name, not its id: the id changes with every
  release, the name does not, so the history runs on through releases. Services
  are kept per env and per VM.
- A measurement is stamped with the second the collector took it. A reading that
  measured nothing — a stopped container, stats that did not read — leaves no
  record.
- The files are `metrics/<env>/<vm>/<YYYYMMDD>.<metric>` in the
  [data folder](data-folder.md): one per VM, UTC day and metric, in a binary format
  of this service's own (described in [frame.rs](../docker-statistics-hub/src/metrics_storage/frame.rs)).
  A record is 20 bytes, so a service polled every 3 seconds takes about 3 MB a day
  across its five metrics.
- Days older than `metrics_days_to_keep` (default `7`) are deleted, checked every
  10 minutes. The default is some 2 GB for a hundred services.

`GET /api/service-metrics?env&vm&service&metric&from&to` reads it back: `from`
(inclusive) and `to` (exclusive) are unix seconds, at most 48 hours apart;
`service` is the container name as the UI shows it; `metric` is one of the five
above. The caller needs access to the env. The answer is three columns of one
length, in time order — an interval nothing was measured in is three empty ones:

```json
{ "time": [1791763200, 1791763203], "value": [0.5, 1.5], "limit": [8.0, null] }
```

`limit` is `null` where the value has none.
