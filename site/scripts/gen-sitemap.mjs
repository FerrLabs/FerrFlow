import { writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const outDir = join(root, 'dist/analog/public');
const SITE = 'https://ferrflow.com';

const LEGAL = ['legal', 'privacy', 'security', 'terms', 'cookies'];

function docSlugs(dir) {
  const base = join(root, 'src/content', dir);
  const out = [];
  const walk = (d) => {
    let entries;
    try {
      entries = readdirSync(d, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      const full = join(d, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith('.md'))
        out.push(relative(base, full).replace(/\\/g, '/').replace(/\.md$/, ''));
    }
  };
  walk(base);
  return out;
}

const DOC_DIRS = [
  { dir: 'docs-en', prefix: '/docs/' },
  { dir: 'docs-fr', prefix: '/fr/docs/' },
  { dir: 'docs-v4', prefix: '/v4/docs/' },
  { dir: 'docs-v3', prefix: '/v3/docs/' },
  { dir: 'docs-v2', prefix: '/v2/docs/' },
  { dir: 'docs-v1', prefix: '/v1/docs/' },
  { dir: 'docs-v0', prefix: '/v0/docs/' },
  { dir: 'docs-fr-v4', prefix: '/fr/v4/docs/' },
];

const docPaths = DOC_DIRS.flatMap(({ dir, prefix }) => docSlugs(dir).map((s) => `${prefix}${s}/`));

const paths = [
  '/',
  '/fr/',
  '/performance/',
  '/fr/performance/',
  ...LEGAL.map((p) => `/${p}/`),
  ...LEGAL.map((p) => `/fr/${p}/`),
  ...docPaths,
];

const urls = paths.map((p) => `  <url><loc>${SITE}${p}</loc></url>`).join('\n');

const xml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;

mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, 'sitemap.xml'), xml, 'utf8');
console.log(`[sitemap] wrote ${paths.length} urls to /sitemap.xml`);
