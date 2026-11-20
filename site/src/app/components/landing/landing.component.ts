import { ChangeDetectionStrategy, Component, inject, LOCALE_ID } from '@angular/core';
import { resolveLocale, withLocaleBase } from '@ferrlabs/ui-ng';
import { benchmarks } from '../../data/benchmarks';
import { CopyCommandComponent } from './copy-command.component';
import { HeroTerminalComponent } from './hero-terminal.component';
import { InstallSwitcherComponent } from './install-switcher.component';
import { speedBenchmark } from './speed-benchmark';

interface HeroStat {
  readonly k: string;
  readonly v: string;
}

interface Fact {
  readonly value: string;
  readonly label: string;
  readonly detail: string;
}

interface FeatureCell {
  readonly key: string;
  readonly title: string;
  readonly body?: string;
  readonly formats?: readonly string[];
}

interface Step {
  readonly title: string;
  readonly body: string;
  readonly command: string;
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

const DEFAULT_RUNS = 30;

@Component({
  selector: 'flr-landing',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [CopyCommandComponent, HeroTerminalComponent, InstallSwitcherComponent],
  templateUrl: './landing.component.html',
  styleUrl: './landing.component.css',
})
export class LandingComponent {
  private readonly locale = resolveLocale(inject(LOCALE_ID));
  private readonly number = new Intl.NumberFormat(this.locale, { maximumFractionDigits: 0 });

  protected readonly version = benchmarks.ferrflow_version.replace(/^ferrflow\s+/, '');
  protected readonly actionMajor = `v${this.version.split('.')[0]}`;

  protected readonly docsHref = withLocaleBase(this.locale, '/docs/introduction/');
  protected readonly quickstartHref = withLocaleBase(this.locale, '/docs/quickstart/');
  protected readonly installDocsHref = withLocaleBase(this.locale, '/docs/installation/');
  protected readonly actionsDocsHref = withLocaleBase(this.locale, '/docs/ci/github-actions/');
  protected readonly performanceHref = withLocaleBase(this.locale, '/performance/');
  protected readonly whyHref = withLocaleBase(this.locale, '/why/');

  protected readonly heroStats: readonly HeroStat[] = [
    { k: $localize`:@@hero.stat.latest:Latest`, v: `v${this.version}` },
    { k: $localize`:@@hero.stat.license:License`, v: 'MIT' },
    { k: $localize`:@@hero.stat.lang:Lang`, v: 'Rust' },
  ];

  protected readonly speed = speedBenchmark();
  protected readonly runs = benchmarks.runs ?? DEFAULT_RUNS;

  protected readonly facts: readonly Fact[] = [
    ...this.speedFact(),
    ...this.footprintFact(),
    {
      value: '14+',
      label: $localize`:@@facts.formats.label:manifest formats bumped natively`,
      detail: $localize`:@@facts.formats.detail:Any language, git-driven`,
    },
    {
      value: '0',
      label: $localize`:@@facts.plugins.label:plugins to install`,
      detail: $localize`:@@features.sub:Single binary. Zero plugins. Drop into any CI and go.`,
    },
  ];

  protected readonly features: readonly FeatureCell[] = [
    {
      key: '01',
      title: $localize`:@@features.01.title:Semantic versioning`,
      body: $localize`:@@features.01.body:Reads conventional commits and determines the right bump. Supports semver, calver, sequential, and zerover.`,
    },
    {
      key: '02',
      title: $localize`:@@features.02.title:Monorepo native`,
      body: $localize`:@@features.02.body:Version multiple packages independently with shared paths, dependency chains, and per-package changelogs.`,
    },
    {
      key: '03',
      title: $localize`:@@features.03.title:Multi-forge`,
      body: $localize`:@@features.03.body:GitHub, GitLab, self-hosted forges. Tags, releases and PR preview comments, natively supported.`,
    },
    {
      key: '04',
      title: $localize`:@@features.04.title:14+ version files`,
      formats: FORMATS,
    },
    {
      key: '05',
      title: $localize`:@@features.05.title:Pre-release channels`,
      body: $localize`:@@features.05.body:Branch-based or flag-based: alpha, beta, rc, dev. Floating tags never moved by pre-releases.`,
    },
    {
      key: '06',
      title: $localize`:@@features.06.title:Open source`,
      body: $localize`:@@features.06.body:MIT. Self-host, fork, audit. The Rust binary handles GitHub OIDC directly, so minimal runners work out of the box.`,
    },
  ];

  protected readonly steps: readonly Step[] = [
    {
      title: $localize`:@@start.init.title:Scaffold the config`,
      body: $localize`:@@start.init.body:Detects your version files and writes a .ferrflow config.`,
      command: 'ferrflow init',
    },
    {
      title: $localize`:@@start.check.title:Preview the release`,
      body: $localize`:@@start.check.body:A dry run: see every bump and tag before anything is touched.`,
      command: 'ferrflow check',
    },
    {
      title: $localize`:@@start.release.title:Run the release`,
      body: $localize`:@@start.release.body:Bumps the files, writes the changelog, commits, tags, pushes and creates the release.`,
      command: 'ferrflow release',
    },
  ];

  private speedFact(): Fact[] {
    if (!this.speed || this.speed.competitorMs === undefined) return [];
    const theirsMs = this.number.format(this.speed.competitorMs);
    const { packages, commitsPerPackage } = this.speed;
    return [
      {
        value: `${this.number.format(this.speed.ferrflowMs)} ms`,
        label: $localize`:@@facts.speed.label:release --dry-run, ${packages}:PACKAGES: packages × ${commitsPerPackage}:COMMITS: commits`,
        detail: $localize`:@@facts.speed.detail:semantic-release: ${theirsMs}:INTERPOLATION: ms`,
      },
    ];
  }

  private footprintFact(): Fact[] {
    const sizes = benchmarks.install_sizes;
    const ours = sizes['ferrflow']?.binary;
    const theirs = sizes['release-please']?.npm;
    if (!ours || !theirs) return [];
    const theirsMb = this.number.format(parseFloat(theirs));
    return [
      {
        value: `${this.number.format(parseFloat(ours))} MB`,
        label: $localize`:@@facts.footprint.label:on disk, native binary`,
        detail: $localize`:@@facts.footprint.detail:release-please: ${theirsMb}:INTERPOLATION: MB via npm`,
      },
    ];
  }
}
