# Federation

When `peers` is set in the master collector's settings, every request that needs
cross-host data **fans out to peers in real time** (parallel HTTP calls,
`peers_request_timeout_secs` per call, default 30s). There is no peer cache and
no sync timer — adding/removing peers in the YAML is picked up on the next
request.

- `GET /api/containers` and `GET /api/containers/running` return the union of
  local containers and a real-time fan-out to every peer's
  `/api/containers/local`. Each item carries an `instance` field naming the
  source's `ENV_INFO`. Failed peers are logged to stderr and skipped
  (best-effort merge).
- `GET /api/containers/logs?id=...&lines_number=...` tries the local Docker
  socket first; if the id is unknown locally it broadcasts the log fetch to all
  peers in parallel and returns the first 200 response.
- `GET /api/containers/local` is the **peer-facing** endpoint — local data only,
  never re-fanouts. This makes A↔B reciprocal peering safe (no recursion).
- The `/mcp` tools behave the same way — searches and log retrieval span the
  fleet through the master.
- `/metrics` aggregation across peers is **not** federated in this version.

## Peer addresses

Each `peers` entry is a URL handed straight to [FlUrl](https://github.com/MyJetTools/fl-url),
so it accepts every form FlUrl does:

| form | example | what happens |
| --- | --- | --- |
| plain | `http://collector-b:8000` | DNS-resolves the host, plain HTTP |
| TLS | `https://collector-c.example.com:8443` | DNS-resolves the host, TLS verified against it |
| known IP | `https://collector-d.example.com@10.0.0.7:8443` | connects straight to `10.0.0.7`, no DNS lookup |

The **known IP** form is `scheme://<server-name>@<ip>[:port]`. The socket goes to
the ip, while everything the peer sees still names `server-name`: the `Host`
header, the TLS SNI, and the name the certificate is checked against. It is the
way to reach a peer whose name does not resolve from the master — a private
address, a host missing from DNS — without giving up certificate verification.

- The part after `@` **must be an IP**, not a host name — a name there would need
  exactly the DNS lookup this form exists to skip. IPv6 goes in brackets:
  `https://collector-d.example.com@[2001:db8::7]:8443`.
- The part before `@` is a bare server name: no port, no `user:password`.
- The port is written after the ip and applies to both, so the `Host` header
  becomes `collector-d.example.com:8443`.
- An entry that breaks these rules is not a peer FlUrl can open — it will not
  appear in the fleet, and the reason goes to stderr.

TLS is backed by rustls with the `ring` crypto provider (fl-url's
`with-ring-tls` feature).
