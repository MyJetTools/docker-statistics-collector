# Documentation

How the docs of this repository are laid out — and the rule to keep when adding
to them:

- **[README.md](../README.md)** says how to **run** the services: what to deploy,
  with which settings and mounts, in what order, and how to check that it works.
  Nothing else. A technical component appears there as a heading and a link —
  never as a description.
- **`docs/`** has one file per technical component, and that file is where the
  component is described in full: what it does, why it is built that way, its
  formats, endpoints and settings.
- The READMEs inside the service folders only point here.

So when something is added:

- a new component — write `docs/<component>.md`, and add its heading and link to
  the list at the end of the README and to the table below;
- a change in how a service is run — a new mount, a setting it cannot start
  without, a renamed image — goes into the README itself.

## Components

| Component | Service | What it covers |
| --- | --- | --- |
| [Collector](collector.md) | collector | How it works, its HTTP API, every setting |
| [File descriptor statistics](file-descriptor-statistics.md) | collector | Open files per container, and the `/proc` mount behind them |
| [Host disk statistics](host-disk-statistics.md) | collector | Disks of the host, and the root mount behind them |
| [Federation](federation.md) | collector | Master and peers, peer addresses, TLS |
| [MCP endpoint](mcp.md) | collector | The tools for AI agents, and the exec unlock |
| [Hub](hub.md) | hub | What it does, its HTTP API, every setting, the rename from `docker-statistics-api` |
| [Access control](access-control.md) | hub | Who sees which environment |
| [Data folder](data-folder.md) | hub | The one folder the hub writes to, and the rule for adding to it |
| [Disk titles](disk-titles.md) | hub, ui | Names given to host disks |
| [Service metrics](service-metrics.md) | hub | The history of every container, on disk by the day |
| [Live logs](live-logs.md) | hub | The log stream over WebSocket |
| [UI](ui.md) | ui | What is on the page, its addresses, what it asks the hub for |
| [Building and releasing](releasing.md) | all | Tags, images, CI, and why a fresh build can break |
