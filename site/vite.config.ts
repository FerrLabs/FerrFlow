import { defineConfig } from 'vite';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import fm from 'front-matter';
import analog from '@analogjs/platform';
import tailwindcss from '@tailwindcss/vite';
import { LOCALES, DEFAULT_LOCALE } from './src/app/i18n/locales';

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
        out.push(relative(base, full).replaceAll('\\', '/').replace(/\.md$/, ''));
    }
  };
  walk(base);
  return out;
}

const changelogDir = join(here, 'src/content/changelog');

function changelogSlugs(): string[] {
  let files: string[];
  try {
    files = readdirSync(changelogDir).filter((f) => f.endsWith('.md'));
  } catch {
    return [];
  }
  return files
    .filter((file) => {
      const raw = readFileSync(join(changelogDir, file), 'utf8');
      const { attributes } = fm<{ draft?: boolean; product?: string }>(raw);
      return !attributes.draft && attributes.product === 'ferrflow';
    })
    .map((file) => file.replace(/\.md$/, ''));
}

const changelogRoutes = changelogSlugs().map((s) => `/changelog/${s}/`);

const LEGAL = ['legal', 'privacy', 'security', 'terms', 'cookies'];

const DOC_DIRS: { dir: string; prefix: string }[] = [
  { dir: 'docs-en', prefix: '/docs/' },
  { dir: 'docs-v6', prefix: '/v6/docs/' },
  { dir: 'docs-v5', prefix: '/v5/docs/' },
  { dir: 'docs-v4', prefix: '/v4/docs/' },
  { dir: 'docs-v3', prefix: '/v3/docs/' },
  { dir: 'docs-v2', prefix: '/v2/docs/' },
  { dir: 'docs-v1', prefix: '/v1/docs/' },
  { dir: 'docs-v0', prefix: '/v0/docs/' },
];

const docRoutes = DOC_DIRS.flatMap(({ dir, prefix }) => docSlugs(dir).map((s) => `${prefix}${s}/`));

// Slugs present per content directory, injected as a build-time constant so
// the docs nav can hide entries a frozen version never shipped. Derived from
// the same walk that feeds the prerender routes, so the two cannot disagree.
const ALL_DOC_DIRS = [
  'docs-en',
  'docs-fr',
  'docs-fr-v6',
  'docs-v6',
  'docs-fr-v5',
  'docs-v5',
  'docs-fr-v4',
  'docs-v4',
  'docs-v3',
  'docs-v2',
  'docs-v1',
  'docs-v0',
];

const docSlugsByDir = Object.fromEntries(ALL_DOC_DIRS.map((dir) => [dir, docSlugs(dir)]));

const prerenderRoutes = [
  '/',
  '/404',
  '/performance/',
  '/why/',
  '/changelog/',
  '/contact/',
  ...LEGAL.map((p) => `/${p}/`),
  ...changelogRoutes,
  ...docRoutes,
];

export default defineConfig({
  define: {
    __DOC_SLUGS_BY_DIR__: JSON.stringify(docSlugsByDir),
  },
  build: {
    target: ['es2022'],
  },
  resolve: {
    dedupe: [
      '@angular/core',
      '@angular/common',
      '@angular/platform-browser',
      '@angular/router',
      '@angular/forms',
    ],
  },
  optimizeDeps: {
    include: ['@ferrlabs/ui-ng', '@ferrlabs/ui-ng/docs'],
  },
  ssr: {
    noExternal: ['@ferrlabs/ui-ng'],
  },
  plugins: [
    analog({
      static: true,
      i18n: {
        defaultLocale: DEFAULT_LOCALE,
        locales: [...LOCALES],
      },
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
