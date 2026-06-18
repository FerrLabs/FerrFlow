import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { DocsLayoutComponent } from '@ferrlabs/ui-ng/docs';
import { Lang } from '../i18n/translations';
import { ShellComponent } from '../components/shell.component';
import { DOC_VERSIONS, DOCS_NAV } from './docs-nav';

@Component({
  selector: 'flr-docs-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ShellComponent, DocsLayoutComponent],
  template: `
    <flr-shell
      [lang]="lang()"
      [title]="metaTitle()"
      [description]="description()"
      [canonical]="canonical()"
      [enHref]="enHref()"
      [frHref]="frHref()"
    >
      <flr-docs-layout
        [nav]="DOCS_NAV"
        [versions]="DOC_VERSIONS"
        [lang]="lang()"
        [slug]="slug()"
        [version]="version()"
      >
        <ng-content />
      </flr-docs-layout>
    </flr-shell>
  `,
})
export class DocsPageComponent {
  readonly lang = input<Lang>('en');
  readonly slug = input.required<string>();
  readonly title = input('');
  readonly description = input('');
  readonly version = input('current');

  protected readonly DOCS_NAV = DOCS_NAV;
  protected readonly DOC_VERSIONS = DOC_VERSIONS;

  private readonly isFr = computed(() => this.lang() === 'fr');

  private readonly versionPath = computed(() =>
    this.version() === 'current' ? '' : `/${this.version()}`,
  );

  protected readonly metaTitle = computed(() => {
    const title = this.title();
    return title ? `${title} — FerrFlow Docs` : 'FerrFlow Docs';
  });

  protected readonly enHref = computed(() => `${this.versionPath()}/docs/${this.slug()}`);
  protected readonly frHref = computed(() => `/fr${this.versionPath()}/docs/${this.slug()}`);
  protected readonly canonical = computed(() =>
    this.isFr()
      ? `https://ferrflow.com/fr${this.versionPath()}/docs/${this.slug()}/`
      : `https://ferrflow.com${this.versionPath()}/docs/${this.slug()}/`,
  );
}
