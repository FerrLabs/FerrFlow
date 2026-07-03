import { ChangeDetectionStrategy, Component, computed, inject, LOCALE_ID } from '@angular/core';
import { resolveLocale, withLocaleBase } from '@ferrlabs/ui-ng';
import { benchmarks } from '../data/benchmarks';

interface HeroStat {
  readonly k: string;
  readonly v: string;
}

interface FeatureCell {
  readonly i: number;
  readonly key: string;
  readonly title: string;
  readonly body: string;
}

interface InstallCell {
  readonly k: string;
  readonly cmd: string;
}

const FORMATS = [
  'Cargo.toml',
  'package.json',
  'pyproject.toml',
  'pom.xml',
  '*.csproj',
  'build.gradle',
  'Chart.yaml',
  'pubspec.yaml',
  'mix.exs',
  '*.gemspec',
  'Package.swift',
  'go.mod',
  'composer.json',
] as const;

@Component({
  selector: 'flr-landing',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section class="hero-section">
      <div class="container">
        <div class="hero-grid">
          <div class="hero-left">
            <h1 class="hero-title">
              <span i18n="@@hero.h1">Release.</span>
              <br />
              <span class="hero-italic" i18n="@@hero.h2">Automatically.</span>
            </h1>
            <p class="hero-sub" i18n="@@hero.sub">
              Conventional commits in, semantic versions out. FerrFlow reads your git history, bumps
              versions across your monorepo, writes changelogs, tags, and ships GitHub releases —
              straight from CI.
            </p>

            <div class="hero-actions">
              <a href="https://github.com/FerrLabs/FerrFlow" class="btn-primary mono">
                <span class="hero-prompt">$</span
                ><span i18n="@@hero.cta1">cargo install ferrflow</span>
              </a>
              <a [href]="docsHref" class="btn-ghost" i18n="@@hero.cta2">Read the docs</a>
            </div>

            <div class="hero-stats">
              @for (s of heroStats(); track s.k) {
                <div>
                  <div class="mono hero-stat-key">{{ s.k }}</div>
                  <div class="mono hero-stat-val">{{ s.v }}</div>
                </div>
              }
            </div>
          </div>

          <div class="terminal">
            <div class="terminal-chrome">
              <span class="terminal-dot terminal-dot-accent"></span>
              <span class="terminal-dot"></span>
              <span class="terminal-dot"></span>
              <span class="terminal-title">~/code/monorepo · ferrflow v4.2</span>
            </div>
            <div class="terminal-body">
              <div class="terminal-line">
                <span class="terminal-prompt">$</span><span>ferrflow release</span>
              </div>
              <div class="terminal-line is-dim">
                <span class="terminal-prompt">→</span
                ><span i18n="@@terminal.reading">reading 184 commits since last release...</span>
              </div>
              <div class="terminal-line is-dim">
                <span class="terminal-prompt">→</span
                ><span i18n="@@terminal.detected">detected: 12 features, 9 fixes, 1 breaking</span>
              </div>
              <div class="terminal-line is-dim">
                <span class="terminal-prompt">→</span
                ><span i18n="@@terminal.scanning">scanning 4 packages...</span>
              </div>
              <div class="terminal-line is-ok">
                <span class="terminal-prompt is-ok">✓</span
                ><span>bump api v4.1.7 → v5.0.0 (breaking)</span>
              </div>
              <div class="terminal-line is-ok">
                <span class="terminal-prompt is-ok">✓</span
                ><span>bump app v4.1.4 → v4.2.0 (minor)</span>
              </div>
              <div class="terminal-line is-ok">
                <span class="terminal-prompt is-ok">✓</span
                ><span>bump site v1.3.9 → v1.4.0 (minor)</span>
              </div>
              <div class="terminal-line is-ok">
                <span class="terminal-prompt is-ok">✓</span
                ><span>bump admin v0.8.2 → v0.8.3 (patch)</span>
              </div>
              <div class="terminal-line is-dim">
                <span class="terminal-prompt">→</span
                ><span i18n="@@terminal.generating"
                  >generating changelogs, tagging, pushing...</span
                >
              </div>
              <div class="terminal-line is-ok">
                <span class="terminal-prompt is-ok">✓</span
                ><span i18n="@@terminal.published">4 GitHub releases published · 0.42s</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="formats-strip">
      <div class="container formats-row">
        <span class="mono formats-eyebrow" i18n="@@formats.label">Reads natively</span>
        <div class="formats-chips">
          @for (f of formats; track f) {
            <span class="mono formats-chip">{{ f }}</span>
          }
        </div>
      </div>
    </section>

    <section id="features" class="features-section">
      <div class="container">
        <p class="mono section-eyebrow" i18n="@@features.eyebrow">№ 02 — Features</p>
        <div class="features-header">
          <h2 class="section-title" i18n="@@features.title">Everything you need to release.</h2>
          <p class="features-sub" i18n="@@features.sub">
            Single binary. Zero plugins. Drop into any CI and go.
          </p>
        </div>

        <div class="features-grid">
          @for (cell of features; track cell.key) {
            <div class="feature-cell" [attr.data-i]="cell.i">
              <span class="mono feature-key">— {{ cell.key }}</span>
              <h3 class="feature-title">{{ cell.title }}</h3>
              <p class="feature-body">{{ cell.body }}</p>
            </div>
          }
        </div>
      </div>
    </section>

    <section id="install" class="install-section">
      <div class="container">
        <p class="mono section-eyebrow" i18n="@@install.eyebrow">№ 03 — Install</p>
        <h2 class="section-title" i18n="@@install.title">Install in seconds.</h2>
        <p class="install-sub" i18n="@@install.sub">
          Works on Linux, macOS, and Windows. Available as a GitHub Action and a GitLab CI template.
        </p>

        <div class="install-grid">
          @for (cell of installCells; track cell.k) {
            <div class="install-cell">
              <div class="mono install-key">{{ cell.k }}</div>
              <code class="mono install-cmd">
                <span class="install-prompt">$</span>{{ cell.cmd }}
              </code>
            </div>
          }
        </div>
      </div>
    </section>
  `,
  styles: `
    .hero-section {
      padding: 80px 0 96px;
    }
    .hero-grid {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      gap: 64px;
      align-items: center;
    }
    .hero-left {
      min-width: 0;
    }
    .hero-title {
      font-family: var(--font-display);
      font-weight: 900;
      font-size: clamp(48px, 6.5vw, 96px);
      line-height: 0.95;
      letter-spacing: -0.04em;
      margin: 0;
      overflow-wrap: break-word;
      hyphens: none;
    }
    .hero-italic {
      font-style: italic;
      font-weight: 400;
      color: var(--accent);
    }
    .hero-sub {
      font-size: 18px;
      line-height: 1.55;
      color: var(--fg-2);
      max-width: 540px;
      margin: 28px 0 0;
    }
    .hero-actions {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      margin-top: 28px;
    }
    .hero-prompt {
      opacity: 0.65;
      margin-right: 6px;
    }
    .hero-stats {
      display: flex;
      gap: 32px;
      flex-wrap: wrap;
      margin-top: 40px;
    }
    .hero-stat-key {
      font-size: 10px;
      letter-spacing: 0.12em;
      text-transform: uppercase;
      color: var(--fg-3);
      margin-bottom: 4px;
    }
    .hero-stat-val {
      font-size: 14px;
      color: var(--fg);
    }

    .terminal {
      background: var(--bg-2);
      border: 1px solid var(--rule);
      border-radius: 14px;
      overflow: hidden;
      font-family: var(--font-mono);
      font-size: 12.5px;
      line-height: 1.7;
      box-shadow: 0 28px 56px -28px rgba(15, 23, 42, 0.22);
    }
    .terminal-chrome {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 12px 16px;
      border-bottom: 1px solid var(--rule);
      font-size: 11px;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--fg-3);
    }
    .terminal-dot {
      width: 10px;
      height: 10px;
      border-radius: 5px;
      background: var(--rule-strong);
    }
    .terminal-dot-accent {
      background: var(--accent);
    }
    .terminal-title {
      margin-left: 12px;
    }
    .terminal-body {
      padding: 20px 22px 26px;
      min-height: 320px;
    }
    .terminal-line {
      display: flex;
      gap: 12px;
      color: var(--fg);
    }
    .terminal-line.is-dim {
      color: var(--fg-3);
    }
    .terminal-line.is-ok {
      color: var(--fg);
    }
    .terminal-prompt {
      color: var(--fg-3);
      min-width: 14px;
    }
    .terminal-prompt.is-ok {
      color: var(--accent);
    }

    @media (max-width: 880px) {
      .hero-grid {
        grid-template-columns: 1fr;
        gap: 48px;
      }
    }

    .formats-strip {
      padding: 32px 0;
      border-top: 1px solid var(--rule);
      border-bottom: 1px solid var(--rule);
    }
    .formats-row {
      display: flex;
      align-items: center;
      gap: 32px;
      flex-wrap: wrap;
    }
    .formats-eyebrow {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--fg-3);
    }
    .formats-chips {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      flex: 1;
    }
    .formats-chip {
      font-size: 12px;
      color: var(--fg-2);
      padding: 6px 12px;
      border-radius: 999px;
      border: 1px solid var(--rule);
      background: var(--card);
    }

    .features-section {
      padding: 96px 0;
    }
    .section-eyebrow {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--fg-3);
      margin: 0 0 28px;
    }
    .section-title {
      font-family: var(--font-display);
      font-weight: 900;
      font-size: clamp(40px, 6vw, 88px);
      line-height: 0.94;
      letter-spacing: -0.04em;
      margin: 0;
    }
    .features-header {
      display: grid;
      grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
      gap: 48px;
      margin-bottom: 64px;
      align-items: end;
    }
    .features-sub {
      font-size: 17px;
      line-height: 1.55;
      color: var(--fg-2);
      margin: 0;
      align-self: end;
    }

    .features-grid {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      border: 1px solid var(--rule);
      border-radius: 18px;
      overflow: hidden;
      background: var(--card);
    }
    .feature-cell {
      padding: 32px;
      min-height: 220px;
      display: flex;
      flex-direction: column;
      border-right: 1px solid var(--rule);
      border-bottom: 1px solid var(--rule);
    }
    .feature-cell[data-i='2'],
    .feature-cell[data-i='5'] {
      border-right: none;
    }
    .feature-cell[data-i='3'],
    .feature-cell[data-i='4'],
    .feature-cell[data-i='5'] {
      border-bottom: none;
    }
    .feature-key {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--accent);
      margin-bottom: 16px;
    }
    .feature-title {
      font-family: var(--font-display);
      font-weight: 700;
      font-size: 24px;
      letter-spacing: -0.02em;
      margin: 0;
    }
    .feature-body {
      font-size: 14.5px;
      line-height: 1.55;
      color: var(--fg-2);
      margin: 12px 0 0;
    }

    @media (max-width: 900px) {
      .features-header {
        grid-template-columns: 1fr;
        gap: 24px;
      }
      .features-grid {
        grid-template-columns: 1fr 1fr;
      }
      .feature-cell {
        border-right: 1px solid var(--rule);
        border-bottom: 1px solid var(--rule);
      }
      .feature-cell[data-i='1'],
      .feature-cell[data-i='3'],
      .feature-cell[data-i='5'] {
        border-right: none;
      }
      .feature-cell[data-i='2'] {
        border-right: 1px solid var(--rule);
      }
      .feature-cell[data-i='4'],
      .feature-cell[data-i='5'] {
        border-bottom: none;
      }
    }
    @media (max-width: 600px) {
      .features-grid {
        grid-template-columns: 1fr;
      }
      .feature-cell {
        border-right: none !important;
        border-bottom: 1px solid var(--rule) !important;
      }
      .feature-cell:last-child {
        border-bottom: none !important;
      }
    }

    .install-section {
      padding: 64px 0 96px;
    }
    .install-sub {
      font-size: 17px;
      color: var(--fg-2);
      max-width: 640px;
      margin: 24px 0 48px;
    }
    .install-grid {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 12px;
    }
    .install-cell {
      background: var(--card);
      border: 1px solid var(--rule);
      border-radius: 14px;
      padding: 20px;
    }
    .install-key {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--fg-3);
      margin-bottom: 12px;
    }
    .install-cmd {
      font-family: var(--font-mono);
      font-size: 13px;
      color: var(--fg);
      display: block;
      word-break: break-all;
    }
    .install-prompt {
      color: var(--accent);
      margin-right: 6px;
    }
    @media (max-width: 900px) {
      .install-grid {
        grid-template-columns: 1fr 1fr;
      }
    }
    @media (max-width: 540px) {
      .install-grid {
        grid-template-columns: 1fr;
      }
    }
  `,
})
export class LandingComponent {
  private readonly locale = resolveLocale(inject(LOCALE_ID));

  protected readonly formats = FORMATS;

  private readonly ferrflowSemver = computed(() =>
    benchmarks.ferrflow_version.replace(/^ferrflow\s+/, ''),
  );

  private readonly ferrflowVersion = computed(() => `v${this.ferrflowSemver()}`);

  private readonly ferrflowMajor = computed(() => `v${this.ferrflowSemver().split('.')[0]}`);

  private readonly ferrflowBinarySize = computed(() =>
    benchmarks.ferrflow_release_size_mb
      ? `~${Math.round(parseFloat(benchmarks.ferrflow_release_size_mb))} MB`
      : `~${Math.round(parseFloat(benchmarks.ferrflow_binary_size_mb))} MB`,
  );

  protected readonly docsHref = withLocaleBase(this.locale, '/docs/introduction/');

  protected readonly heroStats = computed<readonly HeroStat[]>(() => [
    { k: $localize`:@@hero.stat.latest:Latest`, v: this.ferrflowVersion() },
    { k: $localize`:@@hero.stat.license:License`, v: 'MPL-2.0' },
    { k: $localize`:@@hero.stat.lang:Lang`, v: 'Rust' },
    { k: $localize`:@@hero.stat.size:Download`, v: this.ferrflowBinarySize() },
  ]);

  protected readonly features: readonly FeatureCell[] = [
    {
      i: 0,
      key: '01',
      title: $localize`:@@features.01.title:Semantic versioning`,
      body: $localize`:@@features.01.body:Reads conventional commits and determines the right bump. Supports semver, calver, sequential, and zerover.`,
    },
    {
      i: 1,
      key: '02',
      title: $localize`:@@features.02.title:Monorepo native`,
      body: $localize`:@@features.02.body:Version multiple packages independently with shared paths, dependency chains, and per-package changelogs.`,
    },
    {
      i: 2,
      key: '03',
      title: $localize`:@@features.03.title:Multi-forge`,
      body: $localize`:@@features.03.body:GitHub, GitLab, self-hosted forges. Tags, releases, PR preview comments — natively supported.`,
    },
    {
      i: 3,
      key: '04',
      title: $localize`:@@features.04.title:14+ version files`,
      body: $localize`:@@features.04.body:Cargo.toml, package.json, pyproject.toml, Chart.yaml, mix.exs, gemspec, pubspec.yaml — bumped automatically.`,
    },
    {
      i: 4,
      key: '05',
      title: $localize`:@@features.05.title:Pre-release channels`,
      body: $localize`:@@features.05.body:Branch-based or flag-based: alpha, beta, rc, dev. Floating tags never moved by pre-releases.`,
    },
    {
      i: 5,
      key: '06',
      title: $localize`:@@features.06.title:Open source`,
      body: $localize`:@@features.06.body:MPL-2.0. Self-host, fork, audit. The Rust binary handles GitHub OIDC directly — minimal runners work out of the box.`,
    },
  ];

  protected readonly installCells: readonly InstallCell[] = [
    { k: 'Cargo', cmd: 'cargo install ferrflow' },
    { k: 'npm', cmd: 'npm install -D ferrflow' },
    { k: 'Docker', cmd: 'docker run ghcr.io/ferrlabs/ferrflow' },
    { k: 'GitHub', cmd: `uses: FerrLabs/FerrFlow@${this.ferrflowMajor()}` },
  ];
}
