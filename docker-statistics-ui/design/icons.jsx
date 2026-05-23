// icons.jsx — tiny SVG icons (one-stroke, devtools feel)
const Icon = {
  server: (p) => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <rect x="3" y="4"  width="18" height="6" rx="1.5"/>
      <rect x="3" y="14" width="18" height="6" rx="1.5"/>
      <circle cx="7" cy="7" r=".7" fill="currentColor"/>
      <circle cx="7" cy="17" r=".7" fill="currentColor"/>
    </svg>
  ),
  box: (p) => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M3 7l9-4 9 4-9 4-9-4z"/>
      <path d="M3 7v10l9 4 9-4V7"/>
      <path d="M12 11v10"/>
    </svg>
  ),
  cpu: (p) => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <rect x="6" y="6" width="12" height="12" rx="1.5"/>
      <rect x="9" y="9" width="6" height="6"/>
      <path d="M3 9h2M3 13h2M19 9h2M19 13h2M9 3v2M13 3v2M9 19v2M13 19v2"/>
    </svg>
  ),
  mem: (p) => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <rect x="3" y="7" width="18" height="10" rx="1.5"/>
      <path d="M7 7v10M11 7v10M15 7v10M19 7v10"/>
    </svg>
  ),
  search: (p) => (
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <circle cx="11" cy="11" r="7"/>
      <path d="M21 21l-4.3-4.3"/>
    </svg>
  ),
  bell: (p) => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M6 8a6 6 0 1 1 12 0c0 7 3 9 3 9H3s3-2 3-9z"/>
      <path d="M10 21a2 2 0 0 0 4 0"/>
    </svg>
  ),
  refresh: (p) => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M21 12a9 9 0 1 1-3-6.7"/>
      <path d="M21 4v5h-5"/>
    </svg>
  ),
  ports: (p) => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <rect x="3" y="5" width="18" height="14" rx="2"/>
      <path d="M7 5v14M11 5v14M15 5v14M19 5v14"/>
    </svg>
  ),
  drive: (p) => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M3 17l3-9h12l3 9"/>
      <rect x="3" y="17" width="18" height="4" rx="1"/>
      <circle cx="17" cy="19" r=".7" fill="currentColor"/>
    </svg>
  ),
  copy: (p) => (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <rect x="8" y="8" width="12" height="12" rx="1.5"/>
      <path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/>
    </svg>
  ),
  ext: (p) => (
    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M14 4h6v6"/>
      <path d="M20 4l-9 9"/>
      <path d="M19 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5"/>
    </svg>
  ),
  play: (p) => (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" {...p}>
      <path d="M7 4v16l13-8z"/>
    </svg>
  ),
  pause: (p) => (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" {...p}>
      <rect x="6" y="4" width="4" height="16"/>
      <rect x="14" y="4" width="4" height="16"/>
    </svg>
  ),
  terminal: (p) => (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M4 7l4 5-4 5"/>
      <path d="M12 17h8"/>
    </svg>
  ),
  logs: (p) => (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M4 6h16M4 10h16M4 14h10M4 18h13"/>
    </svg>
  ),
  more: (p) => (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" {...p}>
      <circle cx="5" cy="12" r="1.5"/>
      <circle cx="12" cy="12" r="1.5"/>
      <circle cx="19" cy="12" r="1.5"/>
    </svg>
  ),
  caretDown: (p) => (
    <svg width="9" height="9" viewBox="0 0 12 12" fill="currentColor" {...p}>
      <path d="M2 4l4 4 4-4z"/>
    </svg>
  ),
  arrowDown: (p) => (
    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M12 5v14M5 12l7 7 7-7"/>
    </svg>
  ),
};
window.Icon = Icon;
