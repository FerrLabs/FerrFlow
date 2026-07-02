import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import {
  benchmarks,
  type BenchmarkCell,
  type InstallSize,
  type ParallelCell,
} from '../data/benchmarks';
import { PerfChartComponent, type PerfChartConfig } from './perf-chart.component';

const KNOWN_TOOLS = [
  'commit-and-tag-version',
  'standard-version',
  'semantic-release',
  'release-please',
  'changesets',
  'ferrflow',
] as const;

const FIXTURE_LABELS: Record<string, string> = {
  single: 'Single package · 100 commits',
  'mono-small': 'Monorepo · 10 packages × 100 commits',
  'mono-medium': 'Monorepo · 50 packages × 500 commits',
  'mono-100-1k': 'Monorepo · 100 packages × 1k commits',
  'mono-50-5k': 'Monorepo · 50 packages × 5k commits',
  'mono-large': 'Monorepo · 200 packages × 10k commits',
};

const FIXTURE_SHORT: Record<string, string> = {
  single: '1 × 100',
  'mono-small': '10 × 100',
  'mono-medium': '50 × 500',
  'mono-100-1k': '100 × 1k',
  'mono-50-5k': '50 × 5k',
  'mono-large': '200 × 10k',
};

const COMMAND_LABELS: Record<string, string> = {
  check: 'check',
  'release-dry-run': 'release --dry-run',
  tag: 'tag',
  validate: 'validate',
  version: 'version',
};

const TOOL_COLORS: Record<string, string> = {
  ferrflow: '#e8733a',
  changesets: '#94a3b8',
  'commit-and-tag-version': '#b8c2cf',
  'standard-version': '#cbd5e1',
  'semantic-release': '#7c8a9c',
  'release-please': '#a3b1c2',
};

const PREFERRED_METHOD: Record<string, string> = { ferrflow: 'binary' };
const PARALLEL_CMD = 'release-dry-run';

interface ParsedKey {
  readonly fixture: string;
  readonly tool: string;
  readonly method: string;
  readonly cmd: string;
}

function parseKey(key: string): ParsedKey | null {
  for (const tool of KNOWN_TOOLS) {
    const marker = `-${tool}-`;
    const idx = key.indexOf(marker);
    if (idx < 0) continue;
    const fixture = key.slice(0, idx);
    const rest = key.slice(idx + marker.length);
    const [method, ...cmdParts] = rest.split('-');
    return { fixture, tool, method, cmd: cmdParts.join('-') };
  }
  return null;
}

const preferredMethod = (tool: string) => PREFERRED_METHOD[tool] ?? 'npm';

interface Grid {
  readonly cells: Record<string, BenchmarkCell>;
  readonly fixtures: readonly string[];
  readonly tools: readonly string[];
  readonly commands: readonly string[];
}

function buildGrid(): Grid {
  const data = benchmarks.benchmarks;
  const fixtures = new Set<string>();
  const tools = new Set<string>();
  const commands = new Set<string>();
  const comparableCommands = new Set<string>();
  const cells: Record<string, BenchmarkCell> = {};

  for (const [key, cell] of Object.entries(data)) {
    const p = parseKey(key);
    if (!p) continue;
    if (p.method !== preferredMethod(p.tool)) continue;
    fixtures.add(p.fixture);
    tools.add(p.tool);
    if (p.cmd) {
      commands.add(p.cmd);
      if (p.tool !== 'ferrflow') comparableCommands.add(p.cmd);
    }
    cells[`${p.fixture}|${p.tool}|${p.cmd}`] = cell;
  }

  const fixtureOrder = Object.keys(FIXTURE_LABELS);
  const orderedFixtures = Array.from(fixtures).sort(
    (a, b) => fixtureOrder.indexOf(a) - fixtureOrder.indexOf(b),
  );
  const orderedTools = Array.from(tools).sort((a, b) => {
    if (a === 'ferrflow') return -1;
    if (b === 'ferrflow') return 1;
    return a.localeCompare(b);
  });
  const orderedCommands = Array.from(commands)
    .filter((c) => comparableCommands.has(c))
    .sort();

  return { cells, fixtures: orderedFixtures, tools: orderedTools, commands: orderedCommands };
}

interface ParallelRow {
  readonly fixture: string;
  readonly single: BenchmarkCell;
  readonly parallel: ParallelCell;
}

interface InstallRow {
  readonly tool: string;
  readonly npm: string;
  readonly binary: string;
  readonly isOurs: boolean;
}

interface SrTableRow {
  readonly tool: string;
  readonly cells: readonly string[];
}

@Component({
  selector: 'flr-performance',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [PerfChartComponent],
  template: `
    <main class="perf-main">
      <header class="perf-header">
        <p class="eyebrow">Benchmarks</p>
        <h1>How fast is <em>FerrFlow</em>?</h1>
        <p class="lead">
          Side-by-side comparison against the JavaScript ecosystem release tools, measured with
          <a href="https://github.com/sharkdp/hyperfine">hyperfine</a> on the same hardware, the
          same commits, and the same fixtures. Numbers refresh on every FerrFlow release.
        </p>
        <p class="updated">
          Last update: <strong>{{ updated }}</strong> · ferrflow <code>{{ versionLabel }}</code> ·
          binary size <code>{{ binarySize }} MB</code>
        </p>
      </header>

      @if (!hasCompetitors()) {
        <aside class="perf-note">
          <strong>Heads up.</strong> The snapshot below was captured before competitor benchmarks
          landed in CI. Only FerrFlow rows are shown for now. The next release after
          <a href="https://github.com/FerrLabs/FerrFlow/pull/451">FerrFlow#451</a> will populate the
          comparison rows automatically.
        </aside>
      }

      @if (hasChart()) {
        <section class="perf-chart" aria-label="Latency comparison by command">
          <flr-perf-chart [config]="chartConfig()" />
          <table class="sr-only">
            <caption>
              Median latency,
              {{
                defaultCmdLabel()
              }}
            </caption>
            <thead>
              <tr>
                <th>Tool</th>
                @for (f of grid().fixtures; track f) {
                  <th>{{ fixtureShort(f) }}</th>
                }
              </tr>
            </thead>
            <tbody>
              @for (row of srTableRows(); track row.tool) {
                <tr>
                  <th>{{ row.tool }}</th>
                  @for (cell of row.cells; track $index) {
                    <td>{{ cell }}</td>
                  }
                </tr>
              }
            </tbody>
          </table>
        </section>
      }

      <section class="perf-method">
        <h2>How we measure</h2>
        <ul>
          <li>
            <strong>Hyperfine</strong> with 3 warmup runs and 10 timed runs per (fixture × tool ×
            command) cell. Median latency reported.
          </li>
          <li>
            <strong>Peak RSS</strong> via <code>/usr/bin/time -v</code> on a single subsequent run.
          </li>
          <li>
            <strong>Fixtures</strong> are generated from
            <a href="https://github.com/FerrLabs/Fixtures">FerrLabs/Fixtures</a> with deterministic
            conventional-commit history (no flaky inputs).
          </li>
          <li>
            <strong>All commands</strong> are no-op-ish dry runs: <code>ferrflow check</code>,
            <code>semantic-release --dry-run --no-ci</code>, <code>changesets status</code>,
            <code>standard-version --dry-run</code>, etc. So we're comparing the planning cost, not
            the IO cost of actually mutating files.
          </li>
          <li>
            <strong>Single-threaded comparison.</strong> FerrFlow runs with <code>--jobs 1</code> in
            the chart above, so it's measured like the single-threaded JS tools. Its real-world
            parallel numbers are reported separately.
          </li>
          <li>
            <strong>Reproducible</strong> — the full pipeline is in
            <a href="https://github.com/FerrLabs/Benchmarks">FerrLabs/Benchmarks</a>; results land
            as artifacts on every <code>main</code> push and in each release notes.
          </li>
        </ul>
      </section>

      @if (hasParallel()) {
        <section class="perf-section">
          <h2>Parallel mode</h2>
          <p class="lead">
            The comparison pins FerrFlow to a single thread (<code>--jobs 1</code>) so it's
            apples-to-apples with the single-threaded JS tools. In real CI it uses every core —
            here's the same <code>release --dry-run</code> on all {{ runnerCores() }} cores of the
            benchmark runner.
          </p>
          <div class="perf-table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Fixture</th>
                  <th>1 thread</th>
                  <th class="is-ours">{{ runnerCores() }} cores</th>
                  <th>Speedup</th>
                </tr>
              </thead>
              <tbody>
                @for (row of parallelRows(); track row.fixture) {
                  <tr>
                    <td>{{ fixtureLabel(row.fixture) }}</td>
                    <td>{{ fmt(row.single) }}</td>
                    <td class="is-ours">{{ fmt(row.parallel) }}</td>
                    <td>{{ speedup(row.single, row.parallel) }}</td>
                  </tr>
                }
              </tbody>
            </table>
          </div>
        </section>
      }

      @if (hasInstallSizes()) {
        <section class="perf-section">
          <h2>Install footprint</h2>
          <p class="perf-footprint-lead">
            Disk size after install. <code>release-please</code> appears here only — its runtime
            can't be benched offline (it needs the GitHub API to resolve the latest release), so
            footprint is the fair comparison we can publish.
          </p>
          <div class="perf-table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Tool</th>
                  <th>npm install</th>
                  <th>Native binary</th>
                </tr>
              </thead>
              <tbody>
                @for (row of installRows(); track row.tool) {
                  <tr>
                    <th class="fix-label">{{ row.tool }}</th>
                    <td [class.is-ours]="row.isOurs">
                      <span class="lat">{{ row.npm }}</span>
                    </td>
                    <td [class.is-ours]="row.isOurs">
                      <span class="lat">{{ row.binary }}</span>
                    </td>
                  </tr>
                }
              </tbody>
            </table>
          </div>
        </section>
      }

      <section class="perf-footnote">
        <h2>Why FerrFlow ships smaller numbers</h2>
        <ul>
          <li>
            <strong>Native binary.</strong> No Node.js boot, no npm install, no plugin discovery. A
            <code>ferrflow check</code> on a 50-package monorepo finishes before
            <code>node --version</code> would have warmed up.
          </li>
          <li>
            <strong>libgit2 in-process.</strong> Tag enumeration and commit walking happen via
            libgit2 in the same process. Competitors shell out to <code>git</code> per call;
            process-spawn overhead dominates on dense histories.
          </li>
          <li>
            <strong>Single tool, not a plugin tree.</strong> semantic-release pulls a chain of
            analyzers, notes generators, npm publishers and changelog writers — each with its own
            require() cost. FerrFlow's pipeline is one Rust crate.
          </li>
        </ul>
        <p class="cta">
          Numbers updating every release. See the
          <a href="https://github.com/FerrLabs/FerrFlow/releases/latest">latest release notes</a>
          for the full table including baseline delta.
        </p>
      </section>
    </main>
  `,
  styles: `
    .perf-main {
      max-width: 1080px;
      margin: 0 auto;
      padding: 64px 24px 96px;
      color: #1e293b;
    }
    .perf-header {
      margin-bottom: 56px;
    }
    .eyebrow {
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      font-size: 11px;
      letter-spacing: 0.16em;
      text-transform: uppercase;
      color: #64748b;
      margin: 0 0 12px;
    }
    h1 {
      font-size: clamp(40px, 5vw, 64px);
      line-height: 1.05;
      letter-spacing: -0.025em;
      margin: 0 0 20px;
      font-weight: 700;
    }
    h1 em {
      font-style: italic;
      color: #e8733a;
      font-weight: 600;
    }
    .lead {
      font-size: 18px;
      line-height: 1.55;
      color: #475569;
      max-width: 680px;
    }
    .lead a {
      color: #b8470f;
      text-decoration: underline;
      text-decoration-thickness: 1px;
      text-underline-offset: 3px;
    }
    .updated {
      margin-top: 20px;
      font-size: 13px;
      color: #64748b;
    }
    .updated code {
      font-size: 12px;
      background: #f1f5f9;
      padding: 2px 6px;
      border-radius: 3px;
      color: #1e293b;
    }
    .perf-note {
      padding: 18px 22px;
      background: #fef3c7;
      border-left: 3px solid #f59e0b;
      border-radius: 4px;
      font-size: 14px;
      color: #78350f;
      margin: 0 0 48px;
    }
    .perf-note a {
      color: #b45309;
      font-weight: 600;
    }
    .perf-method,
    .perf-footnote {
      margin-bottom: 56px;
    }
    .perf-method h2,
    .perf-section h2,
    .perf-footnote h2 {
      font-size: 24px;
      letter-spacing: -0.015em;
      font-weight: 700;
      margin: 0 0 18px;
    }
    .perf-method h2 code,
    .perf-section h2 code,
    .perf-footnote h2 code {
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      font-size: 20px;
      background: #fef3ec;
      color: #b8470f;
      padding: 2px 10px;
      border-radius: 4px;
    }
    .perf-method ul,
    .perf-footnote ul {
      padding-left: 20px;
      margin: 0 0 16px;
    }
    .perf-method li,
    .perf-footnote li {
      margin-bottom: 10px;
      line-height: 1.6;
      color: #334155;
    }
    .perf-method code,
    .perf-footnote code {
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      font-size: 13px;
      background: #f1f5f9;
      padding: 1px 5px;
      border-radius: 3px;
      color: #1e293b;
    }
    .perf-method a,
    .perf-footnote a {
      color: #b8470f;
      text-decoration: underline;
      text-decoration-thickness: 1px;
      text-underline-offset: 2px;
    }
    .perf-section {
      margin-bottom: 40px;
    }
    .perf-footprint-lead {
      font-size: 14px;
      line-height: 1.55;
      color: #475569;
      margin: 0 0 16px;
      max-width: 720px;
    }
    .perf-footprint-lead code {
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      font-size: 12px;
      background: #f1f5f9;
      padding: 1px 5px;
      border-radius: 3px;
      color: #1e293b;
    }
    .perf-table-wrap {
      overflow-x: auto;
      border: 1px solid #e2e8f0;
      border-radius: 8px;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      font-size: 14px;
    }
    thead th {
      text-align: left;
      padding: 12px 14px;
      background: #f8fafc;
      border-bottom: 1px solid #e2e8f0;
      font-weight: 600;
      color: #475569;
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      font-size: 12px;
      letter-spacing: 0.02em;
    }
    thead th.is-ours {
      color: #e8733a;
    }
    tbody th.fix-label {
      text-align: left;
      padding: 12px 14px;
      font-weight: 500;
      color: #1e293b;
      border-bottom: 1px solid #f1f5f9;
      white-space: nowrap;
    }
    tbody td {
      padding: 12px 14px;
      border-bottom: 1px solid #f1f5f9;
      vertical-align: top;
    }
    tbody tr:last-child th,
    tbody tr:last-child td {
      border-bottom: none;
    }
    tbody td.is-ours {
      background: rgba(232, 115, 58, 0.06);
    }
    .lat {
      display: block;
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      font-weight: 600;
      color: #1e293b;
    }
    td.is-ours .lat {
      color: #b8470f;
    }
    .cta {
      margin-top: 12px;
      font-size: 14px;
      color: #475569;
    }
    .cta a {
      color: #b8470f;
      font-weight: 600;
      text-decoration: underline;
      text-underline-offset: 3px;
    }
    .sr-only {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }
    @media (max-width: 720px) {
      .perf-main {
        padding: 48px 16px 64px;
      }
      table {
        font-size: 13px;
      }
      thead th,
      tbody th.fix-label,
      tbody td {
        padding: 10px 8px;
      }
    }
  `,
})
export class PerformanceComponent {
  protected readonly versionLabel = benchmarks.ferrflow_version.replace('ferrflow ', 'v');
  protected readonly binarySize = benchmarks.ferrflow_binary_size_mb;
  protected readonly updated = new Date(benchmarks.timestamp).toLocaleDateString('en-US', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });

  protected readonly grid = computed(buildGrid);

  protected readonly hasCompetitors = computed(() => this.grid().tools.length > 1);

  protected readonly defaultCmd = computed(() => {
    const cmds = this.grid().commands;
    return cmds.includes('check') ? 'check' : (cmds[0] ?? 'check');
  });

  protected readonly defaultCmdLabel = computed(() => {
    const c = this.defaultCmd();
    return COMMAND_LABELS[c] ?? c;
  });

  protected readonly chartConfig = computed<PerfChartConfig>(() => {
    const { cells, fixtures, tools, commands } = this.grid();
    const data: Record<string, Record<string, Record<string, number>>> = {};
    for (const fix of fixtures) {
      for (const cmd of commands) {
        const row: Record<string, number> = {};
        for (const tool of tools) {
          const cell = cells[`${fix}|${tool}|${cmd}`];
          if (cell) row[tool] = cell.median_ms;
        }
        if (Object.keys(row).length > 0) {
          (data[fix] ??= {})[cmd] = row;
        }
      }
    }
    return {
      data,
      fixtures,
      commands,
      tools,
      fixtureShort: FIXTURE_SHORT,
      commandLabels: COMMAND_LABELS,
      toolColors: TOOL_COLORS,
      defaultCmd: this.defaultCmd(),
    };
  });

  protected readonly hasChart = computed(() => Object.keys(this.chartConfig().data).length > 0);

  protected readonly srTableRows = computed<readonly SrTableRow[]>(() => {
    const { cells, fixtures, tools } = this.grid();
    const cmd = this.defaultCmd();
    return tools.map((tool) => ({
      tool,
      cells: fixtures.map((f) => this.fmt(cells[`${f}|${tool}|${cmd}`])),
    }));
  });

  protected readonly runnerCores = computed(() => benchmarks.runner_cores ?? null);

  protected readonly parallelRows = computed<readonly ParallelRow[]>(() => {
    const parallelData = benchmarks.ferrflow_parallel ?? {};
    const { cells, fixtures } = this.grid();
    const parseParallelKey = (key: string): { fixture: string; cmd: string } | null => {
      for (const fixture of fixtures) {
        if (key.startsWith(`${fixture}-`)) {
          return { fixture, cmd: key.slice(fixture.length + 1) };
        }
      }
      return null;
    };
    return Object.entries(parallelData)
      .map(([key, cell]): ParallelRow | null => {
        const p = parseParallelKey(key);
        if (!p || p.cmd !== PARALLEL_CMD) return null;
        const single = cells[`${p.fixture}|ferrflow|${p.cmd}`];
        if (!single) return null;
        return { fixture: p.fixture, single, parallel: cell };
      })
      .filter((r): r is ParallelRow => r !== null)
      .sort((a, b) => fixtures.indexOf(a.fixture) - fixtures.indexOf(b.fixture));
  });

  protected readonly hasParallel = computed(() => {
    const cores = this.runnerCores();
    return this.parallelRows().length > 0 && cores !== null && cores > 1;
  });

  protected readonly installRows = computed<readonly InstallRow[]>(() => {
    const installSizes: Record<string, InstallSize> = benchmarks.install_sizes ?? {};
    return Object.keys(installSizes)
      .sort((a, b) => {
        if (a === 'ferrflow') return -1;
        if (b === 'ferrflow') return 1;
        return a.localeCompare(b);
      })
      .map((tool) => ({
        tool,
        npm: this.fmtSize(installSizes[tool].npm),
        binary: this.fmtSize(installSizes[tool].binary),
        isOurs: tool === 'ferrflow',
      }));
  });

  protected readonly hasInstallSizes = computed(() => this.installRows().length > 0);

  protected fixtureShort(fixture: string): string {
    return FIXTURE_SHORT[fixture] ?? fixture;
  }

  protected fixtureLabel(fixture: string): string {
    return FIXTURE_LABELS[fixture] ?? fixture;
  }

  protected fmt(cell: { median_ms: number } | undefined): string {
    if (!cell) return '—';
    if (cell.median_ms < 1) return `${(cell.median_ms * 1000).toFixed(0)} µs`;
    if (cell.median_ms < 1000) return `${cell.median_ms.toFixed(0)} ms`;
    return `${(cell.median_ms / 1000).toFixed(2)} s`;
  }

  protected fmtSize(v: string | undefined): string {
    if (!v || v === 'N/A') return '—';
    return `${v} MB`;
  }

  protected speedup(single: BenchmarkCell, parallel: ParallelCell): string {
    if (parallel.median_ms <= 0) return '—';
    return `${(single.median_ms / parallel.median_ms).toFixed(1)}×`;
  }
}
