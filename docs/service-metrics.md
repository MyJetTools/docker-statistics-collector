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

## The machine as a whole

Next to its services every machine has one more series: what its containers came
to in each poll — the figures its card in the UI shows. The same five metrics, in
the same day files, under the name `*`, which no container can have.

| Metric    | Value                              | Limit kept next to it |
| --------- | ---------------------------------- | --------------------- |
| `cpu`     | cpu in use, all containers         | cores of the host     |
| `mem`     | memory in use, all containers      | RAM of the host       |
| `files`   | open file descriptors, all containers | —                  |
| `net-in`  | inbound throughput, all containers | —                     |
| `net-out` | outbound throughput, all containers | —                    |

- It is a sum over the containers, so `mem` is what they use — not what the host
  has in use by everything else as well. The limits of the containers do not add
  up to a limit of the machine, which is why open files have none here.
- A total nothing was added to is not written as a zero: a machine none of whose
  containers measured a metric has no record of it for that poll.
- A record is stamped with the second the poll came at — the readings it sums are
  not of one second.

It costs what one more service does: about 3 MB a day per machine. It is read
with `GET /api/vm-metrics/chart?env&vm&metric&from&to&points`, which answers
exactly as the chart of a service below does. See
[sample.rs](../docker-statistics-hub/src/metrics_storage/sample.rs).

## As a chart

A week of one metric is two hundred thousand measurements, and a chart is a few
hundred pixels wide. So the hub thins the history itself, and the page asks for
it already thinned:

`GET /api/service-metrics/chart?env&vm&service&metric&from&to&points`

The interval — up to 31 days — is cut into no more than `points` stretches of
equal length (`360` when absent, `2000` at most), and every stretch something was
measured in becomes a point:

```json
{ "step": 1680,
  "time": [1791763200, 1791764880],
  "avg": [0.61, 0.58], "min": [0.40, 0.41], "max": [3.57, 0.92],
  "limit": [16.0, 16.0] }
```

- `step` is the seconds a point stands for; `time` is where it starts.
- `avg` is what a line goes through. `min` and `max` are there so that a spike is
  not averaged away: a minute at four cores inside half an hour at one still
  shows as four.
- `limit` is the limit of the latest measurement of the point.
- A stretch nothing was measured in has **no point** — a service that was down is
  a hole in the chart, not a line through zero.

The history is read a day at a time and folded into the points as it is read, so
the memory a request takes does not grow with the interval. A week of a metric
answers in tens of milliseconds and about 10 KB. See
[chart.rs](../docker-statistics-hub/src/metrics_storage/chart.rs).

This is what the [history dialog](ui.md#the-history-of-a-container) of the UI
is drawn from.
