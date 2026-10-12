# Live logs

The log tail of a container, streamed to the browser through the hub.

`WS /ws/logs?env=<env>&id=<container_id>&tail=N`

- `env` — required, must be one configured in `envs`
- `id` — required, full container id
- `tail` — optional initial backfill of N lines (default 200)

The hub opens an upstream WS to `ws://<master>/ws/logs?id&tail` on the env's
master collector, forwards every text/binary frame to the browser, and sends
a Ping every 5 seconds to keep the connection alive during quiet periods.
Closing the browser tab drops both legs cleanly.

Each text frame is one log line as JSON: `{"tp": <stream>, "line": "<text>"}`,
where `tp=1` is stdout, `tp=2` is stderr (docker's multiplexed framing).
