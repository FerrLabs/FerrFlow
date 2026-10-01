import { writeFileSync, readdirSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const outDir = join(here, '..', 'dist/analog/public');
const SITE = 'https://ferrflow.com';

function prerenderedPaths(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return prerenderedPaths(full);
    if (entry.name !== 'index.html') return [];
    const path = relative(outDir, dir).replace(/\\/g, '/');
    return [path ? `/${path}/` : '/'];
  });
}

const paths = prerenderedPaths(outDir)
  .filter((path) => !/^\/(fr\/)?404\//.test(path) && !path.startsWith('/en/'))
  .sort();

if (paths.length === 0) {
  throw new Error(`[sitemap] no prerendered page under ${outDir}`);
}

const urls = paths.map((p) => `  <url><loc>${SITE}${p}</loc></url>`).join('\n');

const xml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;

writeFileSync(join(outDir, 'sitemap.xml'), xml, 'utf8');
console.log(`[sitemap] wrote ${paths.length} urls to /sitemap.xml`);
