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
        <flr-comparison-table
          [eyebrow]="eyebrow"
          [heading]="heading"
          headingAccent="FerrFlow"
          [lead]="lead"
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
      max-width: 1080px;
      margin: 0 auto;
      padding: 64px 24px 96px;
    }
    @media (max-width: 720px) {
      .why-main {
        padding: 48px 16px 64px;
      }
    }
  `,
})
export default class WhyPage {
  protected readonly title = $localize`:@@why.flow.metaTitle:Why FerrFlow — vs semantic-release, release-please, Changesets`;
  protected readonly description = $localize`:@@why.flow.metaDescription:A feature-by-feature comparison of FerrFlow against semantic-release, release-please and Changesets — file formats, monorepos, runtime and release workflow.`;

  protected readonly eyebrow = $localize`:@@why.flow.eyebrow:Comparison`;
  protected readonly heading = $localize`:@@why.flow.h1:Why FerrFlow?`;
  protected readonly lead = $localize`:@@why.flow.lead:A single native binary, language-agnostic across 14+ manifest formats, driven purely by conventional commits. Here is how it lines up against the JavaScript-ecosystem release tools, feature by feature.`;

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
