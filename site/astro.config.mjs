import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';
import starlight from '@astrojs/starlight';
import starlightVersions from 'starlight-versions';

export default defineConfig({
  site: 'https://ferrflow.com',
  output: 'static',
  integrations: [
    starlight({
      title: 'FerrFlow',
      description:
        'FerrFlow — open-source CLI for universal semantic versioning. Bumps versions, generates changelogs, and ships releases across 14+ file formats, monorepos, and CI.',
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
        Header: './src/components/Header.astro',
        PageTitle: './src/components/docs/PageTitle.astro',
        Footer: './src/components/docs/Footer.astro',
      },
      favicon: '/favicon.svg',
      head: [
        { tag: 'link', attrs: { rel: 'icon', href: '/favicon.ico', sizes: '32x32' } },
        { tag: 'link', attrs: { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' } },
        { tag: 'meta', attrs: { property: 'og:image', content: 'https://ferrflow.com/og.png' } },
        { tag: 'meta', attrs: { property: 'og:site_name', content: 'FerrFlow' } },
        { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
        { tag: 'meta', attrs: { name: 'twitter:image', content: 'https://ferrflow.com/og.png' } },
        {
          tag: 'script',
          content: `document.documentElement.dataset.theme='light';try{localStorage.setItem('starlight-theme','light');}catch(e){}`,
        },
        {
          tag: 'script',
          content: `(function(){try{if(localStorage.getItem("lang-pref"))return;var p=location.pathname;if(p.startsWith("/fr/")||p==="/fr")return;var b=(navigator.language||"en").slice(0,2).toLowerCase();if(b==="fr")location.replace("/fr"+(p==="/"?"/":p)+location.search+location.hash);}catch(e){}})();`,
        },
      ],
      social: [],
      customCss: ['./src/styles/claude.css'],
      expressiveCode: {
        themes: ['github-dark-default', 'github-dark-default'],
        useStarlightDarkModeSwitch: false,
        styleOverrides: {
          borderRadius: '12px',
          borderWidth: '1px',
          codeFontFamily: "'DM Mono', ui-monospace, monospace",
          codeFontSize: '0.85rem',
          uiFontFamily:
            "'Inter', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif",
          codeBackground: '#1c1c1c',
          frames: {
            shadowColor: 'transparent',
            editorTabBarBackground: '#161616',
            editorActiveTabBackground: '#1c1c1c',
            terminalBackground: '#1c1c1c',
            terminalTitlebarBackground: '#161616',
          },
        },
      },
      plugins: [
        starlightVersions({
          current: { label: 'v4' },
          versions: [
            { slug: 'v3', label: 'v3' },
            { slug: 'v2', label: 'v2' },
            { slug: 'v1', label: 'v1' },
            { slug: 'v0', label: 'v0' },
          ],
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
          items: [{ label: 'Self-hosting', slug: 'docs/self-hosting' }],
        },
        {
          label: 'Legal',
          items: [{ label: 'Telemetry', slug: 'docs/legal/telemetry' }],
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
