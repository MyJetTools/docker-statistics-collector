# Access control

Who may see which environment. It is decided by the hub, from its settings; the
identity it is decided for comes from the reverse proxy.

## Where the identity comes from

The hub never authenticates anybody. The reverse proxy in front of it does, and
puts the result into the `x-ssl-user` request header — typically the name on the
client certificate. The hub takes that header as it is; an empty or absent one is
"no user". So the hub must only be reachable through that proxy: anyone who can
reach it directly can claim any identity.

See [auth.rs](../docker-statistics-hub/src/auth.rs).

## The rules

They are the `users` and `user_groups` of the hub's settings
(`~/.docker-statistics-hub`):

```yaml
envs:
  prod:
    url: http://collector-master-prod:8000
  staging:
    url: http://collector-master-staging:8000
  dev:
    url: http://collector-master-dev:8000

users:
  alice@example.com: admins        # the "admins" group below
  contractor@vendor.com: dev-only
  ceo@example.com: "*"             # sees every env

user_groups:
  admins: [prod, staging, dev]
  dev-only: [dev]
```

- `users` is not there at all → no access control: every caller sees every env.
  That is the development mode.
- `users` is there:
  - a caller not listed in it sees no env at all;
  - a user of the special group `*` sees all of them;
  - any other user sees the envs of `user_groups[their group]` that are also
    configured in `envs`. A group missing from `user_groups` is no envs.

See [settings_reader.rs](../docker-statistics-hub/src/settings_reader.rs).

## What it covers

The check is the same everywhere an env is named: the REST endpoints, the
[live log](live-logs.md) and exec WebSockets, [disk titles](disk-titles.md) and
[service metrics](service-metrics.md). A user who cannot see an env can neither
read its metrics, nor subscribe to its logs, nor name its disks — the answer is
`403`. `GET /api/envs` lists what the caller may see, and tells the UI who the
caller is.
