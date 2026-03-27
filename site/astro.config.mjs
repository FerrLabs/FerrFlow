import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';
import starlight from '@astrojs/starlight';
import starlightVersions from 'starlight-versions';

export default defineConfig({
  output: 'static',
  integrations: [
    starlight({
      title: 'FerrFlow',
      description: 'Universal semantic versioning for monorepos and classic repos',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/FerrFlow-Org/FerrFlow' },
      ],
      customCss: ['./src/styles/docs.css'],
      expressiveCode: {
        styleOverrides: {
          borderRadius: '0.375rem',
          codeFontFamily: "'DM Mono', ui-monospace, monospace",
          uiFontFamily: "'Fraunces', Georgia, serif",
        },
      },
      plugins: [
        starlightVersions({
          versions: [{ slug: 'v0', label: 'v0' }],
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
            { label: 'ferrflow.toml', slug: 'docs/configuration/config-file' },
            { label: 'Monorepo', slug: 'docs/configuration/monorepo' },
            { label: 'Supported formats', slug: 'docs/configuration/formats' },
          ],
        },
        {
          label: 'CI / CD',
          items: [
            { label: 'GitHub Actions', slug: 'docs/ci/github-actions' },
            { label: 'GitLab CI', slug: 'docs/ci/gitlab-ci' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'CLI commands', slug: 'docs/reference/cli' },
            { label: 'Conventional Commits', slug: 'docs/reference/conventional-commits' },
          ],
        },
      ],
    }),
  ],
  vite: {
    plugins: [tailwindcss()],
  },
  server: {
    port: 4321,
  },
});
