import { ChangeDetectionStrategy, Component, effect, inject, input } from '@angular/core';
import { DOCUMENT } from '@angular/common';
import { Meta, Title } from '@angular/platform-browser';
import { Lang } from '../i18n/translations';
import { SiteNavbarComponent } from './navbar.component';
import { SiteFooterComponent } from './footer.component';

@Component({
  selector: 'flr-shell',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteNavbarComponent, SiteFooterComponent],
  template: `
    <flr-site-navbar
      [lang]="lang()"
      [enHref]="enHref()"
      [frHref]="frHref()"
      [enAvailable]="enAvailable()"
      [frAvailable]="frAvailable()"
    />
    <main><ng-content /></main>
    <flr-site-footer [lang]="lang()" />
  `,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      min-height: 100dvh;
    }
    main {
      flex: 1 0 auto;
    }
  `,
})
export class ShellComponent {
  readonly lang = input<Lang>('en');
  readonly title = input('');
  readonly description = input('');
  readonly canonical = input<string | null>(null);
  readonly enHref = input('/');
  readonly frHref = input('/fr/');
  readonly enAvailable = input(true);
  readonly frAvailable = input(true);

  private readonly titleService = inject(Title);
  private readonly meta = inject(Meta);
  private readonly document = inject(DOCUMENT);

  constructor() {
    effect(() => {
      const title = this.title();
      const description = this.description();
      const canonical = this.canonical();
      if (title) {
        this.titleService.setTitle(title);
        this.meta.updateTag({ property: 'og:title', content: title });
        this.meta.updateTag({ name: 'twitter:title', content: title });
      }
      if (description) {
        this.meta.updateTag({ name: 'description', content: description });
        this.meta.updateTag({ property: 'og:description', content: description });
        this.meta.updateTag({ name: 'twitter:description', content: description });
      }
      if (canonical) {
        this.setCanonical(canonical);
        this.meta.updateTag({ property: 'og:url', content: canonical });
      }
    });
  }

  private setCanonical(href: string): void {
    const head = this.document.head;
    let link = head.querySelector<HTMLLinkElement>('link[rel="canonical"]');
    if (!link) {
      link = this.document.createElement('link');
      link.setAttribute('rel', 'canonical');
      head.appendChild(link);
    }
    link.setAttribute('href', href);
  }
}
