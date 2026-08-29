import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  LOCALE_ID,
} from '@angular/core';
import { DocsLayoutComponent } from '@ferrlabs/ui-ng/docs';
import { resolveLocale, SiteShellComponent } from '@ferrlabs/ui-ng';
import { DOC_VERSIONS, DOCS_NAV } from './docs-nav';
import { navForVersion } from './docs-availability';

@Component({
  selector: 'flr-docs-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent, DocsLayoutComponent],
  template: `
    <flr-site-shell [title]="metaTitle()" [description]="description()">
      <flr-docs-layout
        [nav]="nav()"
        [versions]="DOC_VERSIONS"
        [lang]="locale"
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

  protected readonly nav = computed(() => navForVersion(DOCS_NAV, this.version(), this.locale));

  protected readonly metaTitle = computed(() => {
    const title = this.title();
    return title ? `${title}: FerrFlow Docs` : 'FerrFlow Docs';
  });
}
