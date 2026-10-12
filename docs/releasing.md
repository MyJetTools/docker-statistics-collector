# Building and releasing

## One release per service

Each service is released on its own, by a tag of its own:

| Service   | Tag                                     | Image                                            | Workflow |
| --------- | --------------------------------------- | ------------------------------------------------ | -------- |
| collector | `docker-statistics-collector-<version>` | `ghcr.io/myjettools/docker-statistics-collector` | [release-docker-statistics-collector.yaml](../.github/workflows/release-docker-statistics-collector.yaml) |
| hub       | `docker-statistics-hub-<version>`       | `ghcr.io/myjettools/docker-statistics-hub`       | [release-docker-statistics-hub.yaml](../.github/workflows/release-docker-statistics-hub.yaml) |
| ui        | `docker-statistics-ui-<version>`        | `ghcr.io/myjettools/docker-statistics-ui`        | [release-docker-statistics-ui.yaml](../.github/workflows/release-docker-statistics-ui.yaml) |

The version is the part of the tag after the last `-`. The workflow writes it into
`Cargo.toml`, builds, and pushes the image under that version. The `version` in
the repository's `Cargo.toml` files is a placeholder.

Release only what changed, from `main`:

```bash
# the latest version of a service
git tag -l 'docker-statistics-hub-*' --sort=-v:refname | head -1

# the next one
gh release create docker-statistics-hub-0.2.22 --title docker-statistics-hub-0.2.22 --notes "" --target main

# the build it started
gh run list --limit 5
gh run watch <run-id> --exit-status
```

The hub was `docker-statistics-api` up to `0.2.20`; its old tags and images keep
that name — see [Hub](hub.md#formerly-docker-statistics-api).

## Building locally

```bash
# collector, hub
cd docker-statistics-hub && cargo build --release

# ui — needs the Dioxus CLI (dx) and the wasm32-unknown-unknown target
cd docker-statistics-ui && dx bundle --web --release
```

Unit tests: `cargo test` in the folder of a service — the ui included, its logic
compiles and runs on the host.

## A fresh build can break with nobody touching the code

The MyJetTools dependencies are pinned by **git tag**, and no `Cargo.lock` is
committed. Those tags are moved upstream to newer commits, sometimes with a
changed API — so a build that resolves them afresh, which is what CI does, can
fail on code that compiled at the last release.

- Before changing a service, build it untouched. An error in a dependency's API
  there is this, not the toolchain.
- A `Cargo.lock` left by an earlier local build hides it. To see what CI will
  see, build from a copy of the tree without the lock files.
- The fix is to adapt the call sites to the current API of the dependency and
  commit that on its own.
