import type { BenchmarkCell } from '../data/benchmarks';

export type SeriesByFixture = Record<string, Record<string, Record<string, number>>>;

export interface ChartSeries {
  readonly data: SeriesByFixture;
  readonly startup: SeriesByFixture;
}

export function formatMs(value: number | null): string {
  if (value == null) return '';
  if (value < 1000) return `${Math.round(value)} ms`;
  return `${(value / 1000).toFixed(2)} s`;
}

function collectRows(
  cells: Readonly<Record<string, BenchmarkCell>>,
  fixture: string,
  command: string,
  tools: readonly string[],
): { median: Record<string, number>; startup: Record<string, number> } {
  const median: Record<string, number> = {};
  const startup: Record<string, number> = {};
  for (const tool of tools) {
    const cell = cells[`${fixture}|${tool}|${command}`];
    if (!cell) continue;
    median[tool] = cell.median_ms;
    if (cell.startup_ms !== undefined) startup[tool] = cell.startup_ms;
  }
  return { median, startup };
}

function addRow(
  target: SeriesByFixture,
  fixture: string,
  command: string,
  row: Record<string, number>,
): void {
  if (Object.keys(row).length === 0) return;
  const byCommand = target[fixture] ?? {};
  byCommand[command] = row;
  target[fixture] = byCommand;
}

export function buildSeries(
  cells: Readonly<Record<string, BenchmarkCell>>,
  fixtures: readonly string[],
  commands: readonly string[],
  tools: readonly string[],
): ChartSeries {
  const data: SeriesByFixture = {};
  const startup: SeriesByFixture = {};
  for (const fixture of fixtures) {
    for (const command of commands) {
      const rows = collectRows(cells, fixture, command, tools);
      addRow(data, fixture, command, rows.median);
      addRow(startup, fixture, command, rows.startup);
    }
  }
  return { data, startup };
}
