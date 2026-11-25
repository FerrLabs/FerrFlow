import { readFileSync, readdirSync } from 'node:fs';
import { join, relative } from 'node:path';

export const SITE = 'https://ferrflow.com';

export function prerenderedPaths(outDir, dir = outDir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return prerenderedPaths(outDir, full);
    if (entry.name !== 'index.html') return [];
    const path = relative(outDir, dir).replaceAll('\\', '/');
    return [path ? `/${path}/` : '/'];
  });
}

const CANONICAL = /<link rel="canonical" href="([^"]+)"/;

export function isCanonicalToItself(outDir, path) {
  const html = readFileSync(join(outDir, path, 'index.html'), 'utf8');
  const declared = CANONICAL.exec(html)?.[1];
  return declared === undefined || declared === `${SITE}${path}`;
}

export function sitemapPaths(outDir) {
  return prerenderedPaths(outDir)
    .filter((path) => !/^\/(fr\/)?404\//.test(path) && !path.startsWith('/en/'))
    .filter((path) => isCanonicalToItself(outDir, path))
    .sort();
}

export function sitemapXml(paths) {
  const urls = paths.map((p) => `  <url><loc>${SITE}${p}</loc></url>`).join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;
}
