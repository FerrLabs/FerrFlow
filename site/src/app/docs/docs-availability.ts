import type { DocSection } from '@ferrlabs/ui-ng/docs';

/**
 * Injected by vite.config.ts from the same directory walk that produces the
 * prerender routes: content directory → slugs it contains. A build-time
 * constant rather than a runtime glob, so no markdown reaches the bundle.
 */
declare const __DOC_SLUGS_BY_DIR__: Readonly<Record<string, readonly string[]>>;

const SLUGS_BY_DIR: Readonly<Record<string, readonly string[]>> =
  typeof __DOC_SLUGS_BY_DIR__ === 'undefined' ? {} : __DOC_SLUGS_BY_DIR__;

/**
 * Mirrors what the page components pass to `injectContent`: a locale-specific
 * directory when one exists, otherwise the English one for that version. That
 * fallback is why `fr` + `v3` resolves to `docs-v3`: only `docs-fr-v4` was
 * ever translated.
 */
function contentDirs(version: string, lang: string): readonly string[] {
  if (version === 'current') {
    return lang === 'fr' ? ['docs-fr', 'docs-en'] : ['docs-en'];
  }
  return lang === 'fr' ? [`docs-fr-${version}`, `docs-${version}`] : [`docs-${version}`];
}

/** Slugs that actually have a markdown file for this version and locale. */
export function availableSlugs(version: string, lang: string): ReadonlySet<string> {
  for (const dir of contentDirs(version, lang)) {
    const slugs = SLUGS_BY_DIR[dir];
    if (slugs?.length) return new Set(slugs);
  }
  return new Set<string>();
}

/**
 * Drops nav entries with no page in this version, and sections left empty by
 * that. Archived versions are frozen snapshots, so they legitimately lack
 * pages documented later: advertising those links produced 404s (three dead
 * links on v2–v4, five on v0–v1).
 *
 * An empty availability set means the constant was not injected, so the nav is
 * returned untouched rather than rendering a docs page with no navigation.
 */
export function navForVersion(
  nav: readonly DocSection[],
  version: string,
  lang: string,
): readonly DocSection[] {
  const slugs = availableSlugs(version, lang);
  if (slugs.size === 0) return nav;

  return nav
    .map((section) => ({
      ...section,
      items: section.items.filter((item) => slugs.has(item.slug)),
    }))
    .filter((section) => section.items.length > 0);
}
