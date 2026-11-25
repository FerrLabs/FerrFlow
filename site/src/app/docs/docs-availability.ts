import type { DocSection } from '@ferrlabs/ui-ng/docs';
import { availableSlugs as slugsFor, navForVersion as navFor } from './docs-slugs';

/**
 * Injected by vite.config.ts from the same directory walk that produces the
 * prerender routes: content directory → slugs it contains. A build-time
 * constant rather than a runtime glob, so no markdown reaches the bundle.
 */
declare const __DOC_SLUGS_BY_DIR__: Readonly<Record<string, readonly string[]>>;

export const availableSlugs = (version: string, lang: string): ReadonlySet<string> =>
  slugsFor(__DOC_SLUGS_BY_DIR__, version, lang);

export const navForVersion = (
  nav: readonly DocSection[],
  version: string,
  lang: string,
): readonly DocSection[] => navFor(__DOC_SLUGS_BY_DIR__, nav, version, lang);
