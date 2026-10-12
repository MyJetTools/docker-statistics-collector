# UI

`docker-statistics-ui` is the web page: a single-page app compiled to WebAssembly
with [Dioxus](https://dioxuslabs.com/) and served as static files. It has no
server side and no settings of its own — everything it shows comes from the
[hub](hub.md), which it calls on the address it was opened at. How to run it is in
the [README](../README.md#3-ui).

## What is on the page

A top bar over three columns.

**Top bar** — the environment picker (the envs the hub says the caller may see),
the path to what is open, totals of the env (hosts, containers, running, issues),
a "stale" mark once the hub's data is ten seconds old, the time zone every
timestamp is shown in (the browser's), the user as the hub sees them, and the
dark / light switch. The theme is remembered in the browser; before the first
switch it follows the system.

**Hosts (left)** — a card per host: cores and CPU, network in / out, memory in
use against the RAM of the host and against what its containers were promised,
and its disks. Hosts are grouped by name — `prod` in it is Production, `stage` is
Staging, the rest is Dev / CI. "All VMs" is the whole env as one. A click on the
icon of a disk gives the disk a name — see [Disk titles](disk-titles.md).

**Containers (middle)** — the containers of the selected host, or of all of them.
The box filters by name, image, id and labels, and is kept in the address as
`?service=`; the chips filter by state. A row shows state, memory against the
limit, CPU, network and the disk the container wrote. A container without a
memory limit is measured against the RAM of its host. Memory from 80% of the
limit is marked, from 90% in red.

**Detail (right)** — depends on what is picked:

- *A host, or all of them, and no container* — the boards. **Top by CPU** and
  **Top by Memory** side by side; memory ranks either by bytes or by how full the
  container's limit is. Under them, each across the full width: **Top by Open
  files** — the file descriptors the main process of every container holds
  open, and under each count how much of its `nofile` limit that is; a row is
  marked from 80% of the limit and in red from 90%, and the header counts every
  container that close to its limit, shown or cut off (see
  [File descriptor statistics](file-descriptor-statistics.md)); **Top by Disk** —
  every container's writable layer, what it wrote on top of its image, volumes
  not counted; and **Host disks**, the hosts' physical filesystems, fullest
  first, amber from 75% used and red from 90%. One box sets the rows per board
  (10; `0` is all). The container boards follow the filter and the chips; the
  host disks do not. In "All VMs" a row of Host disks leads to its host.
- *A container* — its state, uptime, id, image, stack, when it was created and
  last started; graphs of CPU, memory and network in / out over the last minutes;
  ports and mounts; the tail of its log, live; the `docker-compose.yaml` it was
  started from, when it carries one; the switch of the
  [exec unlock](mcp.md#the-exec-unlock) of its host; its labels. Buttons open the
  whole log, the processes inside the container, and a console; the name of a
  graph opens [its history](#the-history-of-a-container).

## The history of a container

The graphs on the panel of a container are the last minutes. The **name of a
graph** — CPU, Memory, Net In, Net Out — opens everything before them: a dialog
with the history the hub keeps on disk (see [Service metrics](service-metrics.md)).
The open files have no graph on the panel, so the name of their figure under the
graphs, **Open FDs**, opens the same dialog on them.

- One metric at a time, picked in a row above the chart: the four of the panel
  and the open files. One period: the last hour, 6 hours, 24 hours, 3 days or 7.
- The line is the average of each point; the band around it runs from the lowest
  to the highest measurement of that point, so a short spike stays visible in a
  week. A stretch the container was down in is a break in the line.
- The limit of the metric is drawn while it is within four times the highest
  value — the memory limit of a container usually is. A limit far above the data,
  as the cores of the host are for most containers, would flatten the line, so it
  is only named among the figures.
- Moving the pointer over the chart reads out the nearest point: when, its
  average, highest and lowest, and the limit. The figures above the chart — last,
  average, peak, limit — say the same of the whole period without the pointer.
- While the dialog is open it asks again every 30 seconds.

CPU is shown here in **cores** in use, which is what the collector measures.

### The history of a machine

The same dialog shows a machine as a whole — what all of its containers came to
(see [Service metrics](service-metrics.md#the-machine-as-a-whole)). Its CPU is
measured against the cores of the host and its memory against the RAM. There are
two ways in:

- on the card of a machine in the left column, the glyph in front of a figure —
  CPU, network, memory — opens the history of that figure. It works for any
  machine without selecting it; the "All VMs" card has none, the hub keeps
  history machine by machine;
- with one machine selected, the names of the boards **Top by CPU**, **Top by
  Memory** and **Top by Open files** open the history of that metric for the
  machine.

The dialog is [service_history/](../docker-statistics-ui/src/views/dialog/service_history/).

## Addresses

A page can be copied and opened as it is:

| Address                    | What it shows                          |
| -------------------------- | -------------------------------------- |
| `/`                        | nothing selected                       |
| `/all`                     | all hosts of the env                   |
| `/vm/<host>`               | one host                               |
| `/vm/<host>/<container>`   | a container of that host               |
| `/all/<host>/<container>`  | a container, with all hosts in the list |

A container is always addressed with its host: the same name can run on several.
Whatever serves the files has to answer every such path with `index.html`.
See [router.rs](../docker-statistics-ui/src/router.rs).

## What it asks the hub for

All of it relative to the page, which is why the reverse proxy has to route
`/api/` and `/ws/` to the hub:

- `GET /api/envs` — once, when the page opens;
- `GET /api/vm_cpu_and_mem?env&selected_vm` — every second; the hosts, and the
  containers of what is selected. The hub itself asks the collectors every 3
  seconds;
- `GET /api/logs`, `GET /api/processes`, `GET /api/exec-permission` and its
  `enable` / `disable` — when their part of the page is opened;
- `WS /ws/logs`, `WS /ws/exec` — the live log and the console;
- `POST /api/disk-title` — naming a disk;
- `GET /api/service-metrics/chart`, `GET /api/vm-metrics/chart` — the history
  dialog, of a container and of a machine.

The calls are in [api/](../docker-statistics-ui/src/api/).

## Development

```bash
cd docker-statistics-ui
dx serve --platform web      # needs the Dioxus CLI
cargo test                   # the logic — ranking, formatting — runs on the host
```

The page needs a hub at `/api/` and `/ws/` of the address it is served from, so
during development something has to proxy those two to one.

The styles are one file, [dockerscope.css](../docker-statistics-ui/public/assets/dockerscope.css),
with the colours as variables for the two themes.

## The bundle and the image

`dx bundle --web --release` puts the site into
`target/dx/docker-statistics-ui/release/web/public`. CI then runs
[build.py](../docker-statistics-ui/build.py) over its `index.html`, which adds a
random `?id=` to every `.wasm`, `.js` and `.css` it names, so a browser does not
keep the files of the previous release.

The image is `myjettools/web-app-host` with the bundle as its `./wwwroot`: a
static host that listens on `8000` and answers an unknown path with
`index.html`. See the [Dockerfile](../docker-statistics-ui/Dockerfile).
