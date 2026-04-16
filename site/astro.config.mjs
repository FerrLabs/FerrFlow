import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';
import starlight from '@astrojs/starlight';
import starlightVersions from 'starlight-versions';
import ferrflowTheme from './src/styles/ferrflow-theme.json' with { type: 'json' };

export default defineConfig({
  site: 'https://ferrflow.com',
  output: 'static',
  integrations: [
    starlight({
      title: 'FerrFlow',
      description: 'Universal semantic versioning for monorepos and classic repos',
      defaultLocale: 'root',
      locales: {
        root: {
          label: 'English',
          lang: 'en',
        },
        fr: {
          label: 'Fran\u00e7ais',
          lang: 'fr',
        },
      },
      components: {
        ThemeSelect: './src/components/ThemeSelect.astro',
        Header: './src/components/Header.astro',
      },
      favicon: '/favicon.svg',
      head: [
        { tag: 'meta', attrs: { name: 'google', content: 'notranslate' } },
        { tag: 'link', attrs: { rel: 'icon', href: '/favicon.ico', sizes: '32x32' } },
        { tag: 'link', attrs: { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' } },
        { tag: 'meta', attrs: { property: 'og:image', content: 'https://ferrflow.com/og.png' } },
        { tag: 'meta', attrs: { property: 'og:site_name', content: 'FerrFlow' } },
        { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
        { tag: 'meta', attrs: { name: 'twitter:image', content: 'https://ferrflow.com/og.png' } },
      ],
      social: [],
      customCss: ['./src/styles/docs.css'],
      expressiveCode: {
        styleOverrides: {
          borderRadius: '1rem',
          codeFontFamily: "'DM Mono', ui-monospace, monospace",
          uiFontFamily: "'Fraunces', Georgia, serif",
          frames: {
            shadowColor: 'rgba(232, 115, 58, 0.15)',
          },
        },
        themes: [ferrflowTheme],
      },
      plugins: [
        starlightVersions({
          current: { label: 'v2' },
          versions: [{ slug: 'v1', label: 'v1' }, { slug: 'v0', label: 'v0' }],
        }),
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Introduction', slug: 'docs/introduction' },
            { label: 'Installation', slug: 'docs/installation' },
            { label: 'Quick start', slug: 'docs/quickstart' },
          ],
        },
        {
          label: 'Configuration',
          items: [
            { label: 'Config reference', slug: 'docs/configuration/config-file' },
            { label: 'Monorepo', slug: 'docs/configuration/monorepo' },
            { label: 'Supported formats', slug: 'docs/configuration/formats' },
          ],
        },
        {
          label: 'CI / CD',
          items: [
            { label: 'Pipeline Triggers', slug: 'docs/ci/pipeline-triggers' },
            { label: 'GitHub Actions', slug: 'docs/ci/github-actions' },
            { label: 'GitLab CI', slug: 'docs/ci/gitlab-ci' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'CLI commands', slug: 'docs/reference/cli' },
            { label: 'Conventional Commits', slug: 'docs/reference/conventional-commits' },
            { label: 'Error codes', slug: 'docs/reference/errors' },
          ],
        },
        {
          label: 'Deployment',
          items: [
            { label: 'Self-hosting', slug: 'docs/self-hosting' },
          ],
        },
        {
          label: 'Legal',
          items: [
            { label: 'Telemetry', slug: 'docs/legal/telemetry' },
          ],
        },
      ],
    }),
  ],
  redirects: {
    '/en': '/',
    '/en/[...slug]': '/[...slug]',
  },
  vite: {
    plugins: [tailwindcss()],
  },
  server: {
    port: 4321,
  },
});
