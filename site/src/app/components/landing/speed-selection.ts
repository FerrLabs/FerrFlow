export const COMPETITOR = 'semantic-release';

export interface TimedCell {
  readonly median_ms: number;
}

export interface FixtureShape {
  readonly packages: number;
  readonly commitsPerPackage: number;
}

export interface SpeedBenchmark extends FixtureShape {
  readonly competitor: string;
  readonly ferrflowMs: number;
  readonly competitorMs: number | undefined;
}

export function selectSpeedBenchmark(
  cells: Readonly<Record<string, TimedCell>>,
  fixture: string,
  shape: FixtureShape,
): SpeedBenchmark | undefined {
  const ours = cells[`${fixture}-ferrflow-binary-release-dry-run`];
  if (!ours) return undefined;
  const theirs = cells[`${fixture}-${COMPETITOR}-npm-release-dry-run`];
  const ferrflowMs = Math.round(ours.median_ms);
  const competitorMs = theirs ? Math.round(theirs.median_ms) : undefined;
  return {
    ...shape,
    competitor: COMPETITOR,
    ferrflowMs,
    competitorMs:
      competitorMs !== undefined && competitorMs > ferrflowMs ? competitorMs : undefined,
  };
}
