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
  readonly startup: Record<string, Record<string, Record<string, number>>>;
  readonly fixtures: readonly string[];
  readonly commands: readonly string[];
  readonly tools: readonly string[];
  readonly fixtureShort: Record<string, string>;
  readonly commandLabels: Record<string, string>;
  readonly toolColors: Record<string, string>;
  readonly defaultCmd: string;
  readonly runs: number;
}

const FF_ACCENT = '#e8733a';
const STARTUP_FILL = '#e2e8f0';

@Component({
  selector: 'flr-perf-chart',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (config().commands.length > 1) {
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
    }
    <div class="perf-legend">
      @for (tool of config().tools; track tool) {
        <span class="perf-leg">
          <span class="perf-leg-dot" [style.background]="colorFor(tool)"></span>{{ tool }}
        </span>
      }
      @if (isSplit()) {
        <span class="perf-leg">
          <span class="perf-leg-dot is-startup"></span
          ><span i18n="@@perf.chart.legStartup">startup</span>
        </span>
      }
    </div>
    <div class="perf-canvas-wrap">
      <canvas
        #canvas
        role="img"
        i18n-aria-label="@@perf.chart.ariaLabel"
        aria-label="Grouped bar chart of median latency per fixture per tool, linear scale"
      ></canvas>
    </div>
    @if (isSplit()) {
      <p class="perf-chart-note" i18n="@@perf.chart.noteSplit">
        Median of {{ config().runs }} hyperfine runs · linear scale · cold cache · FerrFlow pinned
        to <code>--jobs 1</code> · lower is better. Each bar is startup (process spawn, runtime
        boot, module loading) plus the work itself, measured against a one-package one-commit floor
        fixture. The total is what you wait for; the split is where it goes.
      </p>
    } @else {
      <p class="perf-chart-note" i18n="@@perf.chart.note">
        Median of {{ config().runs }} hyperfine runs · linear scale · cold cache · FerrFlow pinned
        to <code>--jobs 1</code> · lower is better
      </p>
    }
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
    .perf-leg-dot.is-startup {
      background: #e2e8f0;
      border: 1px solid #cbd5e1;
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

  protected isSplit(): boolean {
    return Object.values(this.config().startup).some((byCmd) =>
      Object.values(byCmd).some((byTool) => Object.keys(byTool).length > 0),
    );
  }

  private total(fixture: string, tool: string): number | null {
    return this.config().data[fixture]?.[this.activeCmd]?.[tool] ?? null;
  }

  private startupOf(fixture: string, tool: string): number | null {
    return this.config().startup[fixture]?.[this.activeCmd]?.[tool] ?? null;
  }

  private datasets(): ChartDataset<'bar', (number | null)[]>[] {
    const cfg = this.config();
    if (!this.isSplit()) {
      return cfg.tools.map((tool) => ({
        label: tool,
        data: cfg.fixtures.map((f) => this.total(f, tool)),
        backgroundColor: this.colorFor(tool),
        borderRadius: 3,
      }));
    }
    // Two datasets per tool sharing one stack id: Chart.js then draws one bar
    // per tool split in two, rather than stacking every tool into one column.
    return cfg.tools.flatMap((tool) => [
      {
        label: `${tool} · startup`,
        data: cfg.fixtures.map((f) => this.startupOf(f, tool)),
        backgroundColor: STARTUP_FILL,
        stack: tool,
      },
      {
        label: tool,
        data: cfg.fixtures.map((f) => {
          const total = this.total(f, tool);
          if (total == null) return null;
          const startup = this.startupOf(f, tool);
          return startup == null ? total : Math.max(total - startup, 0);
        }),
        backgroundColor: this.colorFor(tool),
        stack: tool,
        borderRadius: 3,
      },
    ]);
  }

  private build(): void {
    const cfg = this.config();
    const fmt = (v: number | null): string =>
      v == null ? '' : v < 1000 ? `${Math.round(v)} ms` : `${(v / 1000).toFixed(2)} s`;

    // The label above ferrflow's bar reads the total, not the top segment: once
    // the bar is split, dataset 'ferrflow' carries work only.
    const totals = () => cfg.fixtures.map((f) => this.total(f, 'ferrflow'));
    const ffLabels: Plugin<'bar'> = {
      id: 'ffLabels',
      afterDatasetsDraw(chart) {
        const di = chart.data.datasets.findIndex((d) => d.label === 'ferrflow');
        if (di < 0) return;
        const meta = chart.getDatasetMeta(di);
        const values = totals();
        const c = chart.ctx;
        c.save();
        c.fillStyle = FF_ACCENT;
        c.font = `500 11px ${getComputedStyle(document.body).fontFamily}`;
        c.textAlign = 'center';
        meta.data.forEach((bar, i) => {
          const v = values[i];
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
            type: 'linear',
            beginAtZero: true,
            stacked: this.isSplit(),
            ticks: { callback: (v) => fmt(Number(v)) },
          },
          x: { grid: { display: false }, stacked: this.isSplit() },
        },
      },
      plugins: [ffLabels],
    });
  }
}
