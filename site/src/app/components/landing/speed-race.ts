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

const formatters = new Map<string, Intl.NumberFormat>();

function formatterFor(locale: string): Intl.NumberFormat {
  let formatter = formatters.get(locale);
  if (!formatter) {
    formatter = new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });
    formatters.set(locale, formatter);
  }
  return formatter;
}

export function formatMs(ms: number, locale: string): string {
  return `${formatterFor(locale).format(ms)} ms`;
}
