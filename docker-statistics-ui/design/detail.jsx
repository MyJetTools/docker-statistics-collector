// detail.jsx — container detail panel
const { useState: useStateD, useEffect: useEffectD, useMemo: useMemoD, useRef: useRefD } = React;

function fmtMem(mb) {
  if (mb >= 1024) return { v: (mb/1024).toFixed(2), u: "GiB" };
  return { v: mb.toString(), u: "MiB" };
}

function useMetricsHistory(container) {
  // hold rolling buffers of CPU + Mem
  const [hist, setHist] = useStateD(() => {
    const seedCpu = [], seedMem = [];
    let cv = container.cpu || 1;
    let mv = container.mem || 1;
    for (let i = 0; i < 60; i++) {
      cv = Math.max(0.1, Math.min(95, cv + (Math.random() - 0.5) * 8));
      mv = Math.max(8, Math.min(container.memLimit * 0.9, mv + (Math.random() - 0.5) * 30));
      seedCpu.push(cv); seedMem.push(mv);
    }
    return { cpu: seedCpu, mem: seedMem };
  });
  const idRef = useRefD(container.id);

  useEffectD(() => {
    // reset on container change
    if (idRef.current !== container.id) {
      idRef.current = container.id;
      const seedCpu = [], seedMem = [];
      let cv = container.cpu || 1;
      let mv = container.mem || 1;
      for (let i = 0; i < 60; i++) {
        cv = Math.max(0.1, Math.min(95, cv + (Math.random() - 0.5) * 8));
        mv = Math.max(8, Math.min(container.memLimit * 0.9, mv + (Math.random() - 0.5) * 30));
        seedCpu.push(cv); seedMem.push(mv);
      }
      setHist({ cpu: seedCpu, mem: seedMem });
    }
  }, [container.id]);

  useEffectD(() => {
    if (container.state !== "running") return;
    const t = setInterval(() => {
      setHist(h => {
        const c = h.cpu.slice(1);
        const m = h.mem.slice(1);
        const lastC = h.cpu[h.cpu.length - 1];
        const lastM = h.mem[h.mem.length - 1];
        c.push(Math.max(0.2, Math.min(98, lastC + (Math.random() - 0.5) * 9)));
        m.push(Math.max(8, Math.min(container.memLimit * 0.95, lastM + (Math.random() - 0.5) * 36)));
        return { cpu: c, mem: m };
      });
    }, 1500);
    return () => clearInterval(t);
  }, [container.id, container.state, container.memLimit]);

  return hist;
}

function CopyHash({ value, short = 12 }) {
  const [copied, setCopied] = useStateD(false);
  function doCopy(e) {
    e.stopPropagation();
    navigator.clipboard?.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 900);
  }
  return (
    <span className="id-mono" onClick={doCopy} title="copy full id">
      {value.slice(0, short)}
      <Icon.copy className="copy" />
      {copied && <span style={{ color: "var(--accent)", marginLeft: 4 }}>copied</span>}
    </span>
  );
}

function PortsPanel({ ports }) {
  if (!ports || ports.length === 0) {
    return (
      <div className="panel">
        <div className="panel-head"><h3>Ports</h3></div>
        <div style={{ padding: "20px 4px", color: "var(--text-muted)", fontFamily: "var(--mono)", fontSize: 11.5 }}>
          no published ports
        </div>
      </div>
    );
  }
  return (
    <div className="panel">
      <div className="panel-head">
        <h3>Ports</h3>
        <span className="count-pill">{ports.length}</span>
      </div>
      {ports.map((p, i) => (
        <div key={i} className="port-row">
          <span className={"proto " + p.p}>{p.p}</span>
          <span className="mapping">
            <span className="host">0.0.0.0:{p.h}</span>
            <span className="arrow">→</span>
            <span className="container">:{p.c}</span>
          </span>
          <span className="ext">↗ host</span>
          <span className="link" title="open in browser"><Icon.ext /></span>
        </div>
      ))}
    </div>
  );
}

function MountsPanel({ mounts }) {
  if (!mounts || mounts.length === 0) return null;
  return (
    <div className="panel">
      <div className="panel-head">
        <h3>Mounts</h3>
        <span className="count-pill">{mounts.length}</span>
      </div>
      {mounts.map((m, i) => (
        <div key={i} className="mount-row">
          <span className={"type " + m.type}>{m.type}</span>
          <div className="paths">
            <div className="src">{m.src}<span className="rwo">{m.ro ? "ro" : "rw"}</span></div>
            <div className="dst">
              <span className="arrow-down"><Icon.arrowDown /></span>
              {m.dst}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

function LabelsPanel({ container }) {
  const rows = [
    ["container.id", <CopyHash value={container.id} short={20} />],
    ["image", container.image],
    ["command", container.cmd],
    ["network", container.network],
    ["stack", container.stack],
    ["restarts", String(container.restarts)],
    ["pids", String(container.pids)],
    ["env vars", `${container.env} declared`],
    ...Object.entries(container.labels),
  ];
  return (
    <div className="panel">
      <div className="panel-head">
        <h3>Metadata</h3>
        <span className="count-pill">{rows.length} keys</span>
      </div>
      <div className="kv-list">
        {rows.map(([k, v], i) => (
          <div key={i} className="row">
            <span className="k">{k}</span>
            <span className={"v" + (typeof v === "string" && v.length > 60 ? " dim" : "")}>{v}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function LogPreview({ container }) {
  const [lines, setLines] = useStateD(() => generateLogs(container));
  useEffectD(() => { setLines(generateLogs(container)); }, [container.id]);
  useEffectD(() => {
    if (container.state !== "running") return;
    const t = setInterval(() => {
      setLines(ls => {
        const newLine = makeLogLine(container);
        const next = ls.concat(newLine);
        return next.slice(-80);
      });
    }, 2200);
    return () => clearInterval(t);
  }, [container.id, container.state]);

  return (
    <div className="panel">
      <div className="panel-head">
        <h3>Log tail · stdout</h3>
        <span className="count-pill" style={{ color: container.state === "running" ? "var(--accent)" : "var(--text-muted)" }}>
          {container.state === "running" ? "● live" : "○ paused"}
        </span>
      </div>
      <div className="log-mini" ref={el => { if (el) el.scrollTop = el.scrollHeight; }}>
        {lines.map((l, i) => (
          <div key={i} className="line">
            <span className="ts">{l.ts}</span>{" "}
            <span className={"lvl-" + l.lvl}>{l.lvl.toUpperCase().padEnd(5)}</span>{" "}
            <span>{l.msg}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

const LOG_TEMPLATES = {
  nginx: [
    ["info", '"GET / HTTP/1.1" 200 612'],
    ["info", '"GET /api/health HTTP/1.1" 200 21'],
    ["info", '"POST /api/v1/auth/login HTTP/1.1" 204 0'],
    ["warn", "client closed connection while waiting upstream"],
    ["err",  "upstream timed out (110: Connection timed out)"],
  ],
  postgres: [
    ["info", "checkpoint complete: wrote 24 buffers (0.1%)"],
    ["info", "autovacuum: VACUUM public.events"],
    ["warn", "could not receive data from client: Connection reset"],
    ["info", "duration: 12.443 ms  statement: SELECT * FROM users WHERE id=$1"],
  ],
  default: [
    ["info", "request accepted id=req_8d3f trace=1.42ms"],
    ["info", "metric flushed: 184 series → prom"],
    ["ok",   "task complete · processed=312 ok=311 err=1"],
    ["warn", "retrying connection (attempt 2/5)…"],
    ["err",  "panic: runtime error: invalid memory address"],
  ],
};

function makeLogLine(container) {
  const family = container.image.split(":")[0].split("/").pop().split("-")[0];
  const tpl = LOG_TEMPLATES[family] || LOG_TEMPLATES.default;
  const [lvl, msg] = tpl[Math.floor(Math.random() * tpl.length)];
  const d = new Date();
  const ts = d.toISOString().slice(11, 19);
  return { ts, lvl, msg };
}
function generateLogs(container) {
  const out = [];
  for (let i = 0; i < 14; i++) out.push(makeLogLine(container));
  return out;
}

function DetailPanel({ container, accent }) {
  const hist = useMetricsHistory(container || { id: "_", cpu: 1, mem: 1, memLimit: 1024, state: "exited" });
  if (!container) {
    return (
      <main className="detail" style={{ display: "grid", placeItems: "center" }}>
        <div style={{ color: "var(--text-muted)", fontFamily: "var(--mono)", fontSize: 13 }}>
          select a container
        </div>
      </main>
    );
  }
  const cpuNow = hist.cpu[hist.cpu.length - 1];
  const memNow = hist.mem[hist.mem.length - 1];
  const cpuPrev = hist.cpu[hist.cpu.length - 6] || cpuNow;
  const cpuDelta = cpuNow - cpuPrev;
  const memPct = (memNow / container.memLimit) * 100;
  const m = fmtMem(Math.round(memNow));
  const lim = fmtMem(container.memLimit);

  const stateColor = container.state === "running" ? "var(--accent)"
                   : container.state === "restarting" ? "var(--warn)"
                   : container.state === "unhealthy" ? "var(--danger)"
                   : "var(--text-muted)";

  return (
    <main className="detail">
      <div className="hero">
        <div className="top-row">
          <span className="state-pill" style={{
            color: stateColor,
            background: container.state === "running" ? "var(--accent-soft)" : "rgba(255,255,255,.04)",
            borderColor: container.state === "running" ? "rgba(74,222,128,.3)" : "var(--border)"
          }}>
            <span className="dot" />
            {container.state}
          </span>
          <span className="uptime">up {container.uptime}{container.restarts > 0 && ` · ${container.restarts} restarts`}</span>
          <div className="actions">
            <button className="btn"><Icon.terminal /> shell</button>
            <button className="btn"><Icon.logs /> logs</button>
            <button className="btn">{container.state === "running" ? <><Icon.pause /> stop</> : <><Icon.play /> start</>}</button>
            <button className="btn"><Icon.refresh /> restart</button>
            <button className="btn"><Icon.more /></button>
          </div>
        </div>
        <h1>{container.name}</h1>
        <div className="subline">
          <CopyHash value={container.id} short={12} />
          <span className="sep">·</span>
          <span className="img-tag">{container.image}</span>
          <span className="sep">·</span>
          <span><span className="k">stack</span> {container.stack}</span>
          <span className="sep">·</span>
          <span><span className="k">network</span> {container.network}</span>
          <span className="sep">·</span>
          <span><span className="k">created</span> {container.created}</span>
        </div>
      </div>

      <div className="charts-row">
        <div className="chart-card">
          <div className="head">
            <span className="label">
              <span className="sw" style={{ background: "var(--cpu)" }} />
              CPU
            </span>
            <span className="sub">limit {container.cpuLimit || "—"} cores · 2s</span>
          </div>
          <div className="value-row">
            <span className="big">{cpuNow.toFixed(1)}<span className="unit">%</span></span>
            <span className={"delta " + (cpuDelta >= 0 ? "up" : "down")}>
              {cpuDelta >= 0 ? "▲" : "▼"} {Math.abs(cpuDelta).toFixed(1)}%
            </span>
          </div>
          <AreaChart data={hist.cpu} color="var(--cpu)" />
        </div>
        <div className="chart-card">
          <div className="head">
            <span className="label">
              <span className="sw" style={{ background: "var(--mem)" }} />
              Memory
            </span>
            <span className="sub">limit {lim.v} {lim.u} · {memPct.toFixed(0)}% used</span>
          </div>
          <div className="value-row">
            <span className="big">{m.v}<span className="unit">{m.u}</span></span>
            <span className="delta">of {lim.v} {lim.u}</span>
          </div>
          <AreaChart data={hist.mem} color="var(--mem)" unit=" MiB" />
        </div>
      </div>

      <div className="statline">
        <div className="stat">
          <div className="k">Network ▾ rx</div>
          <div className="v">{(Math.random()*12+1).toFixed(1)}<span className="u">MB/s</span></div>
        </div>
        <div className="stat">
          <div className="k">Network ▴ tx</div>
          <div className="v">{(Math.random()*6+.5).toFixed(1)}<span className="u">MB/s</span></div>
        </div>
        <div className="stat">
          <div className="k">Block I/O</div>
          <div className="v">{(Math.random()*60+5).toFixed(0)}<span className="u">MB/s</span></div>
        </div>
        <div className="stat">
          <div className="k">PIDs</div>
          <div className="v">{container.pids}</div>
        </div>
      </div>

      <div className="detail-grid">
        <div>
          <PortsPanel ports={container.ports} />
          <MountsPanel mounts={container.mounts} />
        </div>
        <div>
          <LogPreview container={container} />
          <LabelsPanel container={container} />
        </div>
      </div>
    </main>
  );
}

window.DetailPanel = DetailPanel;
