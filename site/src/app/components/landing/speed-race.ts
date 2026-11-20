const MIN_FERRFLOW_BAR_PERCENT = 1;

export function progressPercent(elapsedMs: number, totalMs: number): number {
  return Math.min(100, (elapsedMs / totalMs) * 100);
}

export function ferrflowBarPercent(ferrflowMs: number, semanticReleaseMs: number): number {
  return Math.max(MIN_FERRFLOW_BAR_PERCENT, progressPercent(ferrflowMs, semanticReleaseMs));
}

export function speedup(ferrflowMs: number, semanticReleaseMs: number): number {
  return Math.round(semanticReleaseMs / ferrflowMs);
}

export function formatMs(ms: number, locale: string): string {
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(ms)} ms`;
}
