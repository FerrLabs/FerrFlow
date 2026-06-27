import { defineConfig } from 'vite';
import { readdirSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
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
        out.push(relative(base, full).replace(/\\/g, '/').replace(/\.md$/, ''));
    }
  };
  walk(base);
  return out;
}

const LEGAL = ['legal', 'privacy', 'security', 'terms', 'cookies'];

const DOC_DIRS: { dir: string; prefix: string }[] = [
  { dir: 'docs-en', prefix: '/docs/' },
  { dir: 'docs-v4', prefix: '/v4/docs/' },
  { dir: 'docs-v3', prefix: '/v3/docs/' },
  { dir: 'docs-v2', prefix: '/v2/docs/' },
  { dir: 'docs-v1', prefix: '/v1/docs/' },
  { dir: 'docs-v0', prefix: '/v0/docs/' },
];

const docRoutes = DOC_DIRS.flatMap(({ dir, prefix }) => docSlugs(dir).map((s) => `${prefix}${s}/`));

const prerenderRoutes = ['/', '/404', '/performance/', ...LEGAL.map((p) => `/${p}/`), ...docRoutes];

export default defineConfig({
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
