import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { sitemapPaths, sitemapXml } from './sitemap.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const outDir = join(here, '..', 'dist/analog/public');

const paths = sitemapPaths(outDir);

if (paths.length === 0) {
  throw new Error(`[sitemap] no prerendered page under ${outDir}`);
}

writeFileSync(join(outDir, 'sitemap.xml'), sitemapXml(paths), 'utf8');
console.log(`[sitemap] wrote ${paths.length} urls to /sitemap.xml`);
