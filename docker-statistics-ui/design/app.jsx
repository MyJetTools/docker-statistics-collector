// app.jsx — main app shell, state, tweaks panel wiring
const { useState, useEffect, useMemo } = React;

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "accent": "green",
  "density": "cozy",
  "showSparklines": true
}/*EDITMODE-END*/;

function App() {
  const fleet = window.FLEET;
  const [activeVmId, setActiveVmId] = useState("vm-prod-app-01");
  const [activeContainerId, setActiveContainerId] = useState(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("all");
  const [tweaks, setTweak] = useTweaks(TWEAK_DEFAULTS);

  const vm = useMemo(() => fleet.find(v => v.id === activeVmId) || fleet[0], [fleet, activeVmId]);

  useEffect(() => {
    const first = vm.containerList.find(c => c.state === "running") || vm.containerList[0];
    setActiveContainerId(first?.id);
    setQuery("");
    setFilter("all");
  }, [vm.id]);

  const container = useMemo(() => vm.containerList.find(c => c.id === activeContainerId), [vm, activeContainerId]);

  // ⌘K focuses search
  useEffect(() => {
    function onKey(e) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        const el = document.querySelector(".search input");
        el && el.focus();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const totals = useMemo(() => {
    let containers = 0, running = 0, unhealthy = 0;
    for (const v of fleet) {
      containers += v.containerList.length;
      for (const c of v.containerList) {
        if (c.state === "running") running++;
        if (c.state === "unhealthy" || c.state === "restarting") unhealthy++;
      }
    }
    return { vms: fleet.length, containers, running, unhealthy };
  }, [fleet]);

  return (
    <div className={"app density-" + tweaks.density} data-accent={tweaks.accent}>
      <header className="topbar">
        <div className="brand">
          <div className="logo">⌬</div>
          <span>dockerscope</span>
          <span className="pulse"></span>
        </div>
        <button className="env">
          <span style={{ color: "var(--accent)" }}>●</span>
          fleet · prod-eu
          <Icon.caretDown className="caret" />
        </button>
        <div className="crumbs">
          <span>fleet</span>
          <span className="sep">/</span>
          <b>{vm.id}</b>
          {container && (
            <React.Fragment>
              <span className="sep">/</span>
              <b style={{ color: "var(--accent)" }}>{container.name}</b>
            </React.Fragment>
          )}
        </div>
        <div className="right">
          <div className="stats">
            <span className="kv"><span className="swatch" style={{ background: "var(--accent)" }} />vms<b>{totals.vms}</b></span>
            <span className="kv"><span className="swatch" style={{ background: "var(--mem)" }} />containers<b>{totals.containers}</b></span>
            <span className="kv"><span className="swatch" style={{ background: "var(--accent)" }} />running<b>{totals.running}</b></span>
            <span className="kv"><span className="swatch" style={{ background: "var(--danger)" }} />issues<b>{totals.unhealthy}</b></span>
          </div>
          <button className="icon-btn" title="refresh"><Icon.refresh /></button>
          <button className="icon-btn" title="notifications"><Icon.bell /></button>
        </div>
      </header>

      <VmRail
        fleet={fleet}
        activeVmId={vm.id}
        onSelect={setActiveVmId}
        showSparklines={tweaks.showSparklines}
      />
      <ContainerList
        vm={vm}
        query={query} setQuery={setQuery}
        filter={filter} setFilter={setFilter}
        activeId={activeContainerId}
        onPick={setActiveContainerId}
      />
      <DetailPanel container={container} />

      <TweaksPanel title="Tweaks">
        <TweakSection label="Accent">
          <TweakColor
            label="color"
            value={tweaks.accent}
            onChange={v => setTweak("accent", v)}
            options={[
              { value: "green", color: "#4ade80" },
              { value: "blue",  color: "#60a5fa" },
              { value: "amber", color: "#f59e0b" },
              { value: "pink",  color: "#ec4899" },
            ]}
          />
        </TweakSection>
        <TweakSection label="Density">
          <TweakRadio
            label="rows"
            value={tweaks.density}
            onChange={v => setTweak("density", v)}
            options={[
              { value: "compact", label: "compact" },
              { value: "cozy",    label: "cozy" },
              { value: "comfy",   label: "comfy" },
            ]}
          />
        </TweakSection>
        <TweakSection label="VM rail">
          <TweakToggle
            label="show sparklines"
            value={tweaks.showSparklines}
            onChange={v => setTweak("showSparklines", v)}
          />
        </TweakSection>
      </TweaksPanel>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
