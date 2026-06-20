import { defineConfig } from 'vite';
import { readdirSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import analog from '@analogjs/platform';
import tailwindcss from '@tailwindcss/vite';

const here = dirname(fileURLToPath(import.meta.url));

function docSlugs(dir: string): string[] {
  const base = join(here, 'src/content', dir);
  const out: string[] = [];
  const walk = (d: string): void => {
    let entries: import('node:fs').Dirent[];
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

const LEGAL = ['legal', 'privacy', 'security', 'terms', 'cookies'];

const DOC_DIRS: { dir: string; prefix: string }[] = [
  { dir: 'docs-en', prefix: '/docs/' },
  { dir: 'docs-fr', prefix: '/fr/docs/' },
  { dir: 'docs-v4', prefix: '/v4/docs/' },
  { dir: 'docs-v3', prefix: '/v3/docs/' },
  { dir: 'docs-v2', prefix: '/v2/docs/' },
  { dir: 'docs-v1', prefix: '/v1/docs/' },
  { dir: 'docs-v0', prefix: '/v0/docs/' },
  { dir: 'docs-fr-v4', prefix: '/fr/v4/docs/' },
];

const docRoutes = DOC_DIRS.flatMap(({ dir, prefix }) =>
  docSlugs(dir).map((s) => `${prefix}${s}/`),
);

const prerenderRoutes = [
  '/',
  '/fr/',
  '/404',
  '/performance/',
  '/fr/performance/',
  ...LEGAL.map((p) => `/${p}/`),
  ...LEGAL.map((p) => `/fr/${p}/`),
  ...docRoutes,
];

export default defineConfig({
  build: {
    target: ['es2022'],
  },
  plugins: [
    analog({
      static: true,
      content: {
        highlighter: 'prism',
      },
      prerender: {
        routes: prerenderRoutes,
      },
    }),
    tailwindcss(),
  ],
});
