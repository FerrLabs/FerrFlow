export interface DocItem {
  readonly label: string;
  readonly slug: string;
}

export interface DocSection {
  readonly label: string;
  readonly items: readonly DocItem[];
}

export const DOCS_NAV: readonly DocSection[] = [
  {
    label: 'Getting Started',
    items: [
      { label: 'Introduction', slug: 'introduction' },
      { label: 'Installation', slug: 'installation' },
      { label: 'Quick start', slug: 'quickstart' },
    ],
  },
  {
    label: 'Configuration',
    items: [
      { label: 'Config reference', slug: 'configuration/config-file' },
      { label: 'Monorepo', slug: 'configuration/monorepo' },
      { label: 'Supported formats', slug: 'configuration/formats' },
    ],
  },
  {
    label: 'CI / CD',
    items: [
      { label: 'Pipeline Triggers', slug: 'ci/pipeline-triggers' },
      { label: 'GitHub Actions', slug: 'ci/github-actions' },
      { label: 'GitLab CI', slug: 'ci/gitlab-ci' },
    ],
  },
  {
    label: 'Reference',
    items: [
      { label: 'CLI commands', slug: 'reference/cli' },
      { label: 'Conventional Commits', slug: 'reference/conventional-commits' },
      { label: 'Error codes', slug: 'reference/errors' },
    ],
  },
  {
    label: 'Deployment',
    items: [{ label: 'Verifying releases', slug: 'verifying-releases' }],
  },
  {
    label: 'Legal',
    items: [{ label: 'Telemetry', slug: 'legal/telemetry' }],
  },
];

export const DOCS_ORDER: readonly DocItem[] = DOCS_NAV.flatMap((s) => s.items);

export const DOC_VERSIONS = [
  { slug: 'current', label: 'v5' },
  { slug: 'v4', label: 'v4' },
  { slug: 'v3', label: 'v3' },
  { slug: 'v2', label: 'v2' },
  { slug: 'v1', label: 'v1' },
  { slug: 'v0', label: 'v0' },
] as const;
