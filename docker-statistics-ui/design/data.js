// data.js — mock fleet of VMs & containers (assigned to window for cross-script use)
(function () {
  const IMAGES = [
    { img: "nginx:1.25-alpine", name: "nginx", ports: [{ h: 8080, c: 80, p: "tcp" }, { h: 8443, c: 443, p: "tcp" }] },
    { img: "postgres:15.4", name: "postgres", ports: [{ h: 5432, c: 5432, p: "tcp" }] },
    { img: "redis:7.2-alpine", name: "redis", ports: [{ h: 6379, c: 6379, p: "tcp" }] },
    { img: "rabbitmq:3.12-management", name: "rabbitmq", ports: [{ h: 5672, c: 5672, p: "tcp" }, { h: 15672, c: 15672, p: "tcp" }] },
    { img: "grafana/grafana:10.2.3", name: "grafana", ports: [{ h: 3000, c: 3000, p: "tcp" }] },
    { img: "prom/prometheus:v2.48.0", name: "prometheus", ports: [{ h: 9090, c: 9090, p: "tcp" }] },
    { img: "elastic/elasticsearch:8.11.1", name: "elastic", ports: [{ h: 9200, c: 9200, p: "tcp" }, { h: 9300, c: 9300, p: "tcp" }] },
    { img: "minio/minio:RELEASE.2024-01-13", name: "minio", ports: [{ h: 9000, c: 9000, p: "tcp" }, { h: 9001, c: 9001, p: "tcp" }] },
    { img: "traefik:v3.0", name: "traefik", ports: [{ h: 80, c: 80, p: "tcp" }, { h: 443, c: 443, p: "tcp" }, { h: 8080, c: 8080, p: "tcp" }] },
    { img: "ghcr.io/acme/api-gateway:1.18.2", name: "api-gateway", ports: [{ h: 4000, c: 4000, p: "tcp" }] },
    { img: "ghcr.io/acme/billing-svc:2.4.0", name: "billing", ports: [{ h: 4101, c: 4101, p: "tcp" }] },
    { img: "ghcr.io/acme/auth-svc:1.9.5", name: "auth", ports: [{ h: 4102, c: 4102, p: "tcp" }] },
    { img: "ghcr.io/acme/notifications:0.8.1", name: "notifs", ports: [{ h: 4103, c: 4103, p: "tcp" }] },
    { img: "ghcr.io/acme/worker-runner:3.1.0", name: "worker", ports: [] },
    { img: "ghcr.io/acme/metrics-collector:0.5.2", name: "metrics", ports: [{ h: 9101, c: 9101, p: "tcp" }] },
    { img: "mongo:7.0", name: "mongo", ports: [{ h: 27017, c: 27017, p: "tcp" }] },
    { img: "clickhouse/clickhouse-server:23.10", name: "clickhouse", ports: [{ h: 8123, c: 8123, p: "tcp" }, { h: 9000, c: 9000, p: "tcp" }] },
    { img: "node:20-alpine", name: "node-worker", ports: [] },
  ];

  const MOUNT_PRESETS = [
    { type: "bind", src: "/etc/nginx/conf.d", dst: "/etc/nginx/conf.d", ro: true },
    { type: "bind", src: "/var/lib/pgdata", dst: "/var/lib/postgresql/data", ro: false },
    { type: "volume", src: "redis-data", dst: "/data", ro: false },
    { type: "bind", src: "/opt/acme/config", dst: "/app/config", ro: true },
    { type: "bind", src: "/var/log/acme", dst: "/var/log/app", ro: false },
    { type: "volume", src: "grafana-storage", dst: "/var/lib/grafana", ro: false },
    { type: "tmpfs", src: "tmpfs", dst: "/tmp", ro: false },
    { type: "bind", src: "/var/run/docker.sock", dst: "/var/run/docker.sock", ro: true },
    { type: "volume", src: "es-data", dst: "/usr/share/elasticsearch/data", ro: false },
    { type: "bind", src: "/srv/minio/data", dst: "/data", ro: false },
    { type: "bind", src: "/srv/letsencrypt", dst: "/letsencrypt", ro: false },
  ];

  const NETWORKS = ["acme-edge", "acme-internal", "monitoring", "bridge"];
  const STACKS = ["acme-prod", "infrastructure", "monitoring", "ingest"];

  function rand(n) { return Math.floor(Math.random() * n); }
  function pick(a) { return a[rand(a.length)]; }
  function hex(n) { let s = ""; for (let i = 0; i < n; i++) s += "0123456789abcdef"[rand(16)]; return s; }
  function uptime() {
    const d = rand(60), h = rand(24), m = rand(60);
    if (d > 0) return `${d}d ${h}h`;
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  }

  function genContainer(vmId, idx, ovr) {
    const tpl = ovr || pick(IMAGES);
    const states = ["running", "running", "running", "running", "running", "running", "exited", "restarting", "unhealthy"];
    const state = pick(states);
    const cpu = state === "running" ? +(Math.random() * 45 + 0.2).toFixed(1) : 0;
    const cpuLimit = pick([0, 0, 1, 2, 4]);
    const memLimitMB = pick([512, 1024, 2048, 4096, 8192]);
    // ~25% of running containers run hot on memory so the feature is visible
    let memMB = state === "running" ? Math.floor(Math.random() * 1800 + 60) : 0;
    if (state === "running" && Math.random() < 0.25) {
      memMB = Math.floor(memLimitMB * (0.8 + Math.random() * 0.18));
    }
    const mounts = [];
    const mountCount = rand(4) + 1;
    for (let i = 0; i < mountCount; i++) mounts.push(pick(MOUNT_PRESETS));
    return {
      id: hex(12),
      vm: vmId,
      name: `${tpl.name}-${vmId.split("-").pop()}-${(idx + 1).toString().padStart(2, "0")}`,
      image: tpl.img,
      state,
      created: `${rand(60) + 1}d ago`,
      uptime: state === "running" ? uptime() : "—",
      cpu,
      cpuLimit,
      mem: memMB,
      memLimit: memLimitMB,
      ports: tpl.ports.slice(),
      mounts,
      network: pick(NETWORKS),
      stack: pick(STACKS),
      restarts: state === "restarting" ? rand(8) + 2 : rand(3),
      pids: rand(80) + 4,
      labels: {
        "com.docker.compose.project": pick(STACKS),
        "com.docker.compose.service": tpl.name,
        "com.docker.compose.version": "2.24.5",
        "org.opencontainers.image.source": `https://github.com/acme/${tpl.name}`,
        "maintainer": "platform@acme.io",
      },
      cmd: `/usr/local/bin/${tpl.name} --config /app/config/${tpl.name}.yaml`,
      env: rand(6) + 4,
    };
  }

  const VMS = [
    { id: "vm-prod-edge-01",  region: "fra1", containers: 12, status: "ok" },
    { id: "vm-prod-edge-02",  region: "fra1", containers: 11, status: "ok" },
    { id: "vm-prod-app-01",   region: "fra1", containers: 18, status: "warn" },
    { id: "vm-prod-app-02",   region: "fra1", containers: 16, status: "ok" },
    { id: "vm-prod-app-03",   region: "ams1", containers: 14, status: "ok" },
    { id: "vm-prod-data-01",  region: "fra1", containers: 7,  status: "ok" },
    { id: "vm-prod-data-02",  region: "fra1", containers: 6,  status: "danger" },
    { id: "vm-stage-app-01",  region: "ams1", containers: 9,  status: "ok" },
    { id: "vm-stage-app-02",  region: "ams1", containers: 8,  status: "ok" },
    { id: "vm-dev-shared-01", region: "nyc3", containers: 22, status: "warn" },
    { id: "vm-dev-shared-02", region: "nyc3", containers: 19, status: "ok" },
    { id: "vm-ci-runner-01",  region: "fra1", containers: 4,  status: "ok" },
  ];

  // assign metrics + containers
  for (const vm of VMS) {
    vm.cpu = +(Math.random() * 60 + 8).toFixed(0);
    vm.mem = +(Math.random() * 70 + 18).toFixed(0);
    vm.diskGB = +(Math.random() * 800 + 80).toFixed(0);
    vm.kernel = pick(["6.5.0-14", "5.15.0-89", "6.1.0-13"]);
    vm.docker = pick(["24.0.7", "25.0.2", "24.0.5"]);
    vm.os = pick(["Ubuntu 22.04", "Debian 12", "Ubuntu 24.04"]);
    vm.ip = `10.${rand(255)}.${rand(255)}.${rand(255)}`;
    vm.containerList = [];
    for (let i = 0; i < vm.containers; i++) {
      vm.containerList.push(genContainer(vm.id, i));
    }
    // ensure at least one nginx + postgres for the main vm
    if (vm.id === "vm-prod-app-01") {
      vm.containerList[0] = genContainer(vm.id, 0, IMAGES[0]); // nginx
      vm.containerList[0].state = "running";
      vm.containerList[0].uptime = uptime();
      vm.containerList[0].cpu = 12.4;
      vm.containerList[0].mem = 184;
      vm.containerList[1] = genContainer(vm.id, 1, IMAGES[1]); // postgres
      vm.containerList[1].state = "running";
      vm.containerList[1].uptime = uptime();
      vm.containerList[2] = genContainer(vm.id, 2, IMAGES[8]); // traefik
      vm.containerList[2].state = "running";
      vm.containerList[2].uptime = uptime();
    }
    // sparkline series
    const series = [];
    let v = vm.cpu;
    for (let i = 0; i < 40; i++) {
      v = Math.max(2, Math.min(98, v + (Math.random() - 0.5) * 12));
      series.push(v);
    }
    vm.spark = series;
  }

  window.FLEET = VMS;
})();
