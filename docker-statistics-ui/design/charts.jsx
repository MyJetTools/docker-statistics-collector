// charts.jsx — sparkline & area chart helpers
const { useMemo, useState, useEffect, useRef } = React;

function buildPath(values, w, h, pad = 2) {
  if (!values || values.length === 0) return { line: "", area: "" };
  const max = Math.max(...values, 1);
  const min = 0;
  const n = values.length;
  const sx = (i) => (i / (n - 1)) * (w - pad * 2) + pad;
  const sy = (v) => h - pad - ((v - min) / (max - min || 1)) * (h - pad * 2);
  let line = "";
  values.forEach((v, i) => {
    const x = sx(i), y = sy(v);
    line += i === 0 ? `M${x.toFixed(2)},${y.toFixed(2)}` : ` L${x.toFixed(2)},${y.toFixed(2)}`;
  });
  const area = `${line} L${sx(n - 1).toFixed(2)},${h} L${sx(0).toFixed(2)},${h} Z`;
  return { line, area };
}

function Sparkline({ data, color = "#4ade80", height = 22 }) {
  const w = 220, h = height;
  const { line, area } = useMemo(() => buildPath(data, w, h), [data]);
  const gradId = useMemo(() => "g" + Math.random().toString(36).slice(2, 8), []);
  return (
    <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none">
      <defs>
        <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.35" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={area} fill={`url(#${gradId})`} />
      <path d={line} fill="none" stroke={color} strokeWidth="1.2" strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}

function AreaChart({ data, color = "#4ade80", height = 92, showGrid = true, unit = "%" }) {
  const wrapRef = useRef(null);
  const [w, setW] = useState(420);
  useEffect(() => {
    if (!wrapRef.current) return;
    const ro = new ResizeObserver((entries) => {
      const cw = entries[0].contentRect.width;
      if (cw > 0) setW(Math.round(cw));
    });
    ro.observe(wrapRef.current);
    return () => ro.disconnect();
  }, []);

  const h = height;
  const { line, area } = useMemo(() => buildPath(data, w, h, 4), [data, w, h]);
  const gradId = useMemo(() => "ag" + Math.random().toString(36).slice(2, 8), []);
  const max = Math.max(...data, 1);
  const lastV = data[data.length - 1] || 0;
  const lastX = ((data.length - 1) / (data.length - 1)) * (w - 8) + 4;
  const lastY = h - 4 - (lastV / (max || 1)) * (h - 8);

  // hover
  const [hover, setHover] = useState(null); // { x, i, v }
  function onMove(e) {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const idx = Math.max(0, Math.min(data.length - 1, Math.round((x / w) * (data.length - 1))));
    const v = data[idx];
    const px = (idx / (data.length - 1)) * (w - 8) + 4;
    const py = h - 4 - (v / (max || 1)) * (h - 8);
    setHover({ x: px, y: py, v, i: idx });
  }
  function onLeave() { setHover(null); }

  return (
    <div ref={wrapRef} style={{ position: "relative", width: "100%" }} onMouseMove={onMove} onMouseLeave={onLeave}>
      <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ height, width: "100%" }}>
        <defs>
          <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity="0.32" />
            <stop offset="100%" stopColor={color} stopOpacity="0" />
          </linearGradient>
        </defs>
        {showGrid && (
          <g>
            {[0.25, 0.5, 0.75].map((p, i) => (
              <line key={i} x1="0" x2={w} y1={h * p} y2={h * p}
                    stroke="#1d232c" strokeDasharray="2 4" strokeWidth="1" />
            ))}
          </g>
        )}
        <path d={area} fill={`url(#${gradId})`} />
        <path d={line} fill="none" stroke={color} strokeWidth="1.4" strokeLinejoin="round" strokeLinecap="round" />
        <circle cx={lastX} cy={lastY} r="3" fill={color} stroke="#0a0b0d" strokeWidth="2" />
        {hover && (
          <g>
            <line x1={hover.x} x2={hover.x} y1="0" y2={h} stroke="#2f3540" strokeWidth="1" />
            <circle cx={hover.x} cy={hover.y} r="3" fill={color} stroke="#0a0b0d" strokeWidth="2" />
          </g>
        )}
      </svg>
      {hover && (
        <div style={{
          position: "absolute",
          left: Math.min(w - 90, Math.max(0, hover.x + 8)),
          top: 0,
          background: "#0a0b0d",
          border: "1px solid #2f3540",
          padding: "4px 8px",
          borderRadius: 4,
          fontFamily: "var(--mono)",
          fontSize: 10.5,
          color: "var(--text)",
          pointerEvents: "none",
          whiteSpace: "nowrap",
        }}>
          <span style={{ color: "var(--text-muted)" }}>t-{(data.length - 1 - hover.i) * 2}s </span>
          <b>{hover.v.toFixed(1)}{unit}</b>
        </div>
      )}
    </div>
  );
}

Object.assign(window, { Sparkline, AreaChart });
