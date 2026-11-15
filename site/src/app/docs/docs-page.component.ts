import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  LOCALE_ID,
} from '@angular/core';
import { DocsLayoutComponent, type DocsLang } from '@ferrlabs/ui-ng/docs';
import { resolveLocale, SiteShellComponent } from '@ferrlabs/ui-ng';
import { DOC_VERSIONS, DOCS_NAV } from './docs-nav';
import { availableSlugs, navForVersion } from './docs-availability';

const ORIGIN = 'https://ferrflow.com';
const VERSIONED_SLUG_PREFIX = /^\/?(?:[a-z]{2}\/)?(?:v\d+\/)?docs\//;

@Component({
  selector: 'flr-docs-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent, DocsLayoutComponent],
  template: `
    <flr-site-shell
      [title]="metaTitle()"
      [description]="description()"
      [canonical]="canonical()"
    >
      <flr-docs-layout
        [nav]="nav()"
        [versions]="DOC_VERSIONS"
        [lang]="docsLang"
        [slug]="slug()"
        [version]="version()"
      >
        <ng-content />
      </flr-docs-layout>
    </flr-site-shell>
  `,
})
export class DocsPageComponent {
  readonly slug = input.required<string>();
  readonly title = input('');
  readonly description = input('');
  readonly version = input('current');

  protected readonly DOC_VERSIONS = DOC_VERSIONS;

  protected readonly locale = resolveLocale(inject(LOCALE_ID));

  protected readonly docsLang: DocsLang = this.locale === 'fr' ? 'fr' : 'en';

  protected readonly nav = computed(() => navForVersion(DOCS_NAV, this.version(), this.locale));

  protected readonly canonical = computed(() => {
    if (this.version() === 'current') return null;
    const slug = this.slug().replace(VERSIONED_SLUG_PREFIX, '');
    if (!availableSlugs('current', this.locale).has(slug)) return null;
    const prefix = this.locale === 'fr' ? '/fr' : '';
    return `${ORIGIN}${prefix}/docs/${slug}/`;
  });

  protected readonly metaTitle = computed(() => {
    const title = this.title();
    return title ? `${title}: FerrFlow Docs` : 'FerrFlow Docs';
  });
}
