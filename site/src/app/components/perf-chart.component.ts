import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  input,
  PLATFORM_ID,
  viewChild,
} from '@angular/core';
import { isPlatformBrowser } from '@angular/common';
import { Chart, registerables, type ChartDataset, type Plugin } from 'chart.js';

export interface PerfChartConfig {
  readonly data: Record<string, Record<string, Record<string, number>>>;
  readonly fixtures: readonly string[];
  readonly commands: readonly string[];
  readonly tools: readonly string[];
  readonly fixtureShort: Record<string, string>;
  readonly commandLabels: Record<string, string>;
  readonly toolColors: Record<string, string>;
  readonly defaultCmd: string;
}

const FF_ACCENT = '#e8733a';

@Component({
  selector: 'flr-perf-chart',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="perf-pills">
      @for (c of config().commands; track c) {
        <button
          type="button"
          class="perf-pill"
          [class.is-active]="c === activeCmd"
          (click)="selectCmd(c)"
        >
          {{ config().commandLabels[c] ?? c }}
        </button>
      }
    </div>
    <div class="perf-legend">
      @for (tool of config().tools; track tool) {
        <span class="perf-leg">
          <span class="perf-leg-dot" [style.background]="colorFor(tool)"></span>{{ tool }}
        </span>
      }
    </div>
    <div class="perf-canvas-wrap">
      <canvas
        #canvas
        role="img"
        aria-label="Grouped bar chart of median latency per fixture per tool, logarithmic scale"
      ></canvas>
    </div>
    <p class="perf-chart-note">
      Median of 10 hyperfine runs · log scale · FerrFlow pinned to <code>--jobs 1</code> · lower is
      better
    </p>
  `,
  styles: `
    :host {
      display: block;
      margin-bottom: 56px;
    }
    .perf-pills {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-bottom: 16px;
    }
    .perf-pill {
      font-family: inherit;
      font-size: 14px;
      padding: 6px 14px;
      border-radius: 999px;
      border: 1px solid #e2e8f0;
      background: #fff;
      color: #475569;
      cursor: pointer;
      transition:
        border-color 0.12s,
        background 0.12s,
        color 0.12s;
    }
    .perf-pill:hover {
      border-color: #cbd5e1;
    }
    .perf-pill.is-active {
      border-color: #e8733a;
      background: rgba(232, 115, 58, 0.1);
      color: #e8733a;
      font-weight: 500;
    }
    .perf-legend {
      display: flex;
      flex-wrap: wrap;
      gap: 16px;
      margin-bottom: 14px;
      font-size: 12px;
      color: #64748b;
    }
    .perf-leg {
      display: flex;
      align-items: center;
      gap: 6px;
    }
    .perf-leg-dot {
      width: 10px;
      height: 10px;
      border-radius: 2px;
      display: inline-block;
    }
    .perf-canvas-wrap {
      position: relative;
      width: 100%;
      height: 380px;
    }
    .perf-chart-note {
      font-size: 12px;
      color: #94a3b8;
      margin: 16px 0 0;
    }
    .perf-chart-note code {
      font-family: var(--font-mono, ui-monospace, monospace);
      font-size: 12px;
      background: #f1f5f9;
      padding: 1px 5px;
      border-radius: 3px;
      color: #1e293b;
    }
  `,
})
export class PerfChartComponent {
  readonly config = input.required<PerfChartConfig>();

  private readonly canvas = viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');

  protected activeCmd = '';
  private chart: Chart | null = null;
  private readonly platformId = inject(PLATFORM_ID);

  constructor() {
    afterNextRender(() => {
      if (!isPlatformBrowser(this.platformId)) return;
      Chart.register(...registerables);
      this.activeCmd = this.config().defaultCmd;
      this.build();
    });
  }

  protected colorFor(tool: string): string {
    return this.config().toolColors[tool] || '#cbd5e1';
  }

  protected selectCmd(cmd: string): void {
    this.activeCmd = cmd;
    if (this.chart) {
      this.chart.data.datasets = this.datasets();
      this.chart.update();
    }
  }

  private datasets(): ChartDataset<'bar', (number | null)[]>[] {
    const cfg = this.config();
    const cmd = this.activeCmd;
    return cfg.tools.map((tool) => ({
      label: tool,
      data: cfg.fixtures.map((f) =>
        cfg.data[f] && cfg.data[f][cmd] ? (cfg.data[f][cmd][tool] ?? null) : null,
      ),
      backgroundColor: this.colorFor(tool),
      borderRadius: 3,
    }));
  }

  private build(): void {
    const cfg = this.config();
    const fmt = (v: number | null): string =>
      v == null ? '' : v < 1000 ? `${Math.round(v)} ms` : `${(v / 1000).toFixed(2)} s`;

    const ffLabels: Plugin<'bar'> = {
      id: 'ffLabels',
      afterDatasetsDraw(chart) {
        const di = chart.data.datasets.findIndex((d) => d.label === 'ferrflow');
        if (di < 0) return;
        const meta = chart.getDatasetMeta(di);
        const c = chart.ctx;
        c.save();
        c.fillStyle = FF_ACCENT;
        c.font = `500 11px ${getComputedStyle(document.body).fontFamily}`;
        c.textAlign = 'center';
        meta.data.forEach((bar, i) => {
          const v = chart.data.datasets[di].data[i] as number | null;
          if (v == null) return;
          c.fillText(fmt(v), bar.x, bar.y - 6);
        });
        c.restore();
      },
    };

    this.chart = new Chart(this.canvas().nativeElement, {
      type: 'bar',
      data: {
        labels: cfg.fixtures.map((f) => cfg.fixtureShort[f] || f),
        datasets: this.datasets(),
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
          tooltip: {
            callbacks: { label: (i) => `${i.dataset.label}: ${fmt(i.parsed.y)}` },
          },
        },
        scales: {
          y: {
            type: 'logarithmic',
            ticks: {
              callback: (v) => {
                const n = Number(v);
                return [1, 10, 100, 1000, 10000].includes(n)
                  ? n < 1000
                    ? `${n} ms`
                    : `${n / 1000} s`
                  : '';
              },
            },
          },
          x: { grid: { display: false } },
        },
      },
      plugins: [ffLabels],
    });
  }
}
