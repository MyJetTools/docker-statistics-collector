// panels.jsx — VM rail + container list components
const { useState: useStateP, useEffect: useEffectP, useMemo: useMemoP } = React;

function VmRail({ fleet, activeVmId, onSelect, showSparklines = true }) {
  const totalsByGroup = useMemoP(() => {
    const groups = { production: [], staging: [], dev: [] };
    for (const vm of fleet) {
      if (vm.id.includes("prod")) groups.production.push(vm);
      else if (vm.id.includes("stage")) groups.staging.push(vm);
      else groups.dev.push(vm);
    }
    return groups;
  }, [fleet]);

  function statusKind(vm) { return vm.status; }

  function renderGroup(label, items, count) {
    return (
      <React.Fragment key={label}>
        <div className="section-label">
          <span>{label}</span>
          <span className="pill">{count}</span>
        </div>
        {items.map(vm => (
          <div
            key={vm.id}
            className={"vm-card" + (vm.id === activeVmId ? " active" : "")}
            onClick={() => onSelect(vm.id)}
          >
            <div className="ico">
              <Icon.server />
              <span className={"heart " + (statusKind(vm) === "ok" ? "" : statusKind(vm))}></span>
            </div>
            <div className="body">
              <div className="name">{vm.id}</div>
              <div className="meta">
                <span className="item">{vm.region}</span>
                <span className="item cpu">{vm.cpu}%</span>
                <span className="item mem">{vm.mem}%</span>
              </div>
            </div>
            <div className="count">{vm.containers}</div>
            {vm.id === activeVmId && showSparklines && (
              <div className="sparkline">
                <Sparkline data={vm.spark} color="var(--accent)" height={22} />
              </div>
            )}
          </div>
        ))}
      </React.Fragment>
    );
  }

  return (
    <aside className="vm-rail">
      {renderGroup("Production", totalsByGroup.production, totalsByGroup.production.length)}
      {renderGroup("Staging", totalsByGroup.staging, totalsByGroup.staging.length)}
      {renderGroup("Dev / CI", totalsByGroup.dev, totalsByGroup.dev.length)}
    </aside>
  );
}

function ContainerList({ vm, query, setQuery, filter, setFilter, activeId, onPick }) {
  const list = useMemoP(() => {
    let l = vm.containerList;
    if (query) {
      const q = query.toLowerCase();
      l = l.filter(c =>
        c.name.toLowerCase().includes(q) ||
        c.image.toLowerCase().includes(q) ||
        c.id.includes(q));
    }
    if (filter !== "all") l = l.filter(c => c.state === filter);
    return l;
  }, [vm, query, filter]);

  const counts = useMemoP(() => {
    const c = { all: vm.containerList.length, running: 0, exited: 0, restarting: 0, unhealthy: 0 };
    vm.containerList.forEach(x => { c[x.state] = (c[x.state] || 0) + 1; });
    return c;
  }, [vm]);

  return (
    <section className="list-col">
      <div className="list-head">
        <div className="title-row">
          <h2>{vm.id}</h2>
          <span className="sub">{vm.containerList.length} containers</span>
        </div>
        <div className="search">
          <Icon.search style={{ color: "var(--text-muted)" }} />
          <input
            placeholder="filter by name, image, id…"
            value={query}
            onChange={e => setQuery(e.target.value)}
          />
          <span className="kbd">⌘K</span>
        </div>
        <div className="filters">
          {[
            ["all", "all"],
            ["running", "running"],
            ["unhealthy", "unhealthy"],
            ["restarting", "restarting"],
            ["exited", "exited"],
          ].map(([k, lbl]) => (
            <button
              key={k}
              className={"chip" + (filter === k ? " active" : "")}
              onClick={() => setFilter(k)}
            >
              <span className="dot" style={{
                background: k === "running" ? "var(--accent)"
                          : k === "exited" ? "var(--text-muted)"
                          : k === "restarting" ? "var(--warn)"
                          : k === "unhealthy" ? "var(--danger)"
                          : "var(--text-dim)"
              }} />
              {lbl} <span style={{ color: "var(--text-muted)", marginLeft: 2 }}>{counts[k] ?? 0}</span>
            </button>
          ))}
        </div>
      </div>
      <div className="list-body">
        {list.map(c => (
          <div
            key={c.id}
            className={"cont-row" + (c.id === activeId ? " active" : "")}
            onClick={() => onPick(c.id)}
          >
            <span className={"state " + (c.state === "running" ? "" : c.state)}></span>
            <div className="info">
              <div className="name">{c.name}</div>
              <div className="image">{c.image}</div>
            </div>
            <div className="metrics">
              <span className="cpu">{c.cpu.toFixed(1)}%</span>
              <span className="mem">{c.mem >= 1024 ? (c.mem/1024).toFixed(1)+"G" : c.mem+"M"}</span>
            </div>
          </div>
        ))}
        {list.length === 0 && (
          <div style={{
            padding: "40px 12px", textAlign: "center",
            fontFamily: "var(--mono)", fontSize: 11.5, color: "var(--text-muted)"
          }}>
            no containers match
          </div>
        )}
      </div>
    </section>
  );
}

Object.assign(window, { VmRail, ContainerList });
