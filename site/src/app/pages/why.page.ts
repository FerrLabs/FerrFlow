import { ChangeDetectionStrategy, Component } from '@angular/core';
import {
  SiteShellComponent,
  ComparisonTableComponent,
  type ComparisonColumn,
  type ComparisonGroup,
} from '@ferrlabs/ui-ng';

@Component({
  selector: 'flr-why',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent, ComparisonTableComponent],
  template: `
    <flr-site-shell [title]="title" [description]="description">
      <main class="why-main">
        <header class="why-header">
          <p class="eyebrow" i18n="@@why.flow.eyebrow">Comparison</p>
          <h1 i18n="@@why.flow.h1">Why <em>FerrFlow</em>?</h1>
          <p class="lead" i18n="@@why.flow.lead">
            A single native binary, language-agnostic across 14+ manifest formats, driven purely by
            conventional commits. Here is how it lines up against the JavaScript-ecosystem release
            tools, feature by feature.
          </p>
        </header>
        <flr-comparison-table
          [columns]="columns"
          [groups]="groups"
          [yesLabel]="yesLabel"
          [noLabel]="noLabel"
          [partialLabel]="partialLabel"
        />
      </main>
    </flr-site-shell>
  `,
  styles: `
    .why-main {
      max-width: 960px;
      margin: 0 auto;
      padding: 64px 24px 96px;
    }
    .why-header {
      margin-bottom: 40px;
    }
    .eyebrow {
      margin: 0 0 8px;
      font-size: 13px;
      font-weight: 600;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--color-primary-600, #e8733a);
    }
    .why-header h1 {
      margin: 0 0 16px;
      font-size: clamp(28px, 5vw, 40px);
      line-height: 1.1;
    }
    .why-header h1 em {
      font-style: normal;
      color: var(--color-primary-600, #e8733a);
    }
    .lead {
      margin: 0;
      max-width: 60ch;
      font-size: 17px;
      line-height: 1.6;
      color: var(--color-ink-2, #475569);
    }
  `,
})
export default class WhyPage {
  protected readonly title = $localize`:@@why.flow.metaTitle:Why FerrFlow — vs semantic-release, release-please, Changesets`;
  protected readonly description = $localize`:@@why.flow.metaDescription:A feature-by-feature comparison of FerrFlow against semantic-release, release-please and Changesets — file formats, monorepos, runtime and release workflow.`;

  protected readonly yesLabel = $localize`:@@why.cell.yes:Yes`;
  protected readonly noLabel = $localize`:@@why.cell.no:No`;
  protected readonly partialLabel = $localize`:@@why.cell.partial:Partial`;

  protected readonly columns: ComparisonColumn[] = [
    { name: 'FerrFlow', ours: true, accent: 'var(--color-primary-600, #e8733a)' },
    { name: 'semantic-release' },
    { name: 'release-please' },
    { name: 'Changesets' },
  ];

  protected readonly groups: ComparisonGroup[] = [
    {
      title: $localize`:@@why.flow.g.commits:Commits & versioning`,
      rows: [
        {
          feature: $localize`:@@why.flow.f.conventional:Conventional-commits driven`,
          cells: [true, true, true, false],
        },
        {
          feature: $localize`:@@why.flow.f.semver:SemVer bump from history`,
          cells: [true, true, true, 'partial'],
        },
        {
          feature: $localize`:@@why.flow.f.nofiles:No changeset files to author`,
          cells: [true, true, true, false],
        },
      ],
    },
    {
      title: $localize`:@@why.flow.g.langs:Language coverage`,
      rows: [
        {
          feature: $localize`:@@why.flow.f.formats:14+ manifest formats`,
          hint: $localize`:@@why.flow.f.formats.hint:Cargo, npm, pyproject, Chart.yaml, mix, gemspec, pubspec…`,
          cells: [true, 'partial', true, false],
        },
        {
          feature: $localize`:@@why.flow.f.monorepo:Monorepo, multi-package`,
          cells: [true, 'partial', true, true],
        },
      ],
    },
    {
      title: $localize`:@@why.flow.g.output:Output`,
      rows: [
        {
          feature: $localize`:@@why.flow.f.changelog:Changelog generation`,
          cells: [true, true, true, true],
        },
        {
          feature: $localize`:@@why.flow.f.release:Git tag + GitHub Release`,
          cells: [true, true, true, true],
        },
        {
          feature: $localize`:@@why.flow.f.releasepr:Release-PR workflow`,
          cells: ['partial', false, true, true],
        },
      ],
    },
    {
      title: $localize`:@@why.flow.g.runtime:Runtime & DX`,
      rows: [
        {
          feature: $localize`:@@why.flow.f.binary:Single static binary, no Node runtime`,
          cells: [true, false, false, false],
        },
        {
          feature: $localize`:@@why.flow.f.native:Native performance, no Node cold start`,
          cells: [true, false, false, false],
        },
        {
          feature: $localize`:@@why.flow.f.action:Ships as a GitHub Action`,
          cells: [true, true, true, true],
        },
        {
          feature: $localize`:@@why.flow.f.oss:Free / open source`,
          cells: [true, true, true, true],
        },
      ],
    },
  ];
}
