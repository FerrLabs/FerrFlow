import Chart from 'chart.js/auto';

type ChartConfig = {
  data: Record<string, Record<string, Record<string, number>>>;
  fixtures: string[];
  commands: string[];
  tools: string[];
  fixtureShort: Record<string, string>;
  commandLabels: Record<string, string>;
  toolColors: Record<string, string>;
  defaultCmd: string;
};

const el = document.getElementById('perf-chart-data');
const ctx = document.getElementById('perf-canvas') as HTMLCanvasElement | null;

if (el && ctx && el.textContent) {
  const cfg = JSON.parse(el.textContent) as ChartConfig;
  let cmd = cfg.defaultCmd;

  const fmt = (v: number | null): string =>
    v == null ? '' : v < 1000 ? `${Math.round(v)} ms` : `${(v / 1000).toFixed(2)} s`;

  const datasets = () =>
    cfg.tools.map((t) => ({
      label: t,
      data: cfg.fixtures.map((f) =>
        cfg.data[f] && cfg.data[f][cmd] ? (cfg.data[f][cmd][t] ?? null) : null,
      ),
      backgroundColor: cfg.toolColors[t] || '#cbd5e1',
      borderRadius: 3,
    }));

  const ffLabels = {
    id: 'ffLabels',
    afterDatasetsDraw(chart: any) {
      const di = chart.data.datasets.findIndex((d: any) => d.label === 'ferrflow');
      if (di < 0) return;
      const meta = chart.getDatasetMeta(di);
      const c = chart.ctx;
      c.save();
      c.fillStyle = '#e8733a';
      c.font = `500 11px ${getComputedStyle(document.body).fontFamily}`;
      c.textAlign = 'center';
      meta.data.forEach((bar: any, i: number) => {
        const v = chart.data.datasets[di].data[i];
        if (v == null) return;
        c.fillText(fmt(v), bar.x, bar.y - 6);
      });
      c.restore();
    },
  };

  const chart = new Chart(ctx, {
    type: 'bar',
    data: { labels: cfg.fixtures.map((f) => cfg.fixtureShort[f] || f), datasets: datasets() },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false },
        tooltip: { callbacks: { label: (i: any) => `${i.dataset.label}: ${fmt(i.parsed.y)}` } },
      },
      scales: {
        y: {
          type: 'logarithmic',
          ticks: {
            callback: (v: any) =>
              [1, 10, 100, 1000, 10000].includes(v) ? (v < 1000 ? `${v} ms` : `${v / 1000} s`) : '',
          },
        },
        x: { grid: { display: false } },
      },
    },
    plugins: [ffLabels],
  });

  const ensureSized = () => {
    if (ctx.parentElement && ctx.parentElement.clientWidth > 0) chart.resize();
    else requestAnimationFrame(ensureSized);
  };
  requestAnimationFrame(ensureSized);
  window.addEventListener('load', () => chart.resize());

  const legend = document.getElementById('perf-legend');
  if (legend) {
    legend.innerHTML = cfg.tools
      .map(
        (t) =>
          `<span class="perf-leg"><span class="perf-leg-dot" style="background:${cfg.toolColors[t] || '#cbd5e1'}"></span>${t}</span>`,
      )
      .join('');
  }

  document.querySelectorAll<HTMLElement>('#perf-cmd [data-cmd]').forEach((b) => {
    b.addEventListener('click', () => {
      cmd = b.getAttribute('data-cmd') || cmd;
      document
        .querySelectorAll<HTMLElement>('#perf-cmd .perf-pill')
        .forEach((p) => p.classList.toggle('is-active', p.getAttribute('data-cmd') === cmd));
      chart.data.datasets = datasets();
      chart.update();
    });
  });
}
