import { ChangeDetectionStrategy, Component, inject, LOCALE_ID } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { injectContent, MarkdownComponent } from '@analogjs/content';
import { SiteShellComponent, resolveLocale, withLocaleBase } from '@ferrlabs/ui-ng';
import { ChangelogAttributes, PRODUCT_META, TYPE_LABEL_LONG } from './changelog.types';

@Component({
  selector: 'flr-changelog-post',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent, MarkdownComponent],
  template: `
    @if (post(); as p) {
      <flr-site-shell [title]="metaTitle(p.attributes.title)" [description]="p.attributes.summary">
        <article class="post">
          <div class="container post__container">
            <p class="mono post__meta">
              <time [attr.datetime]="iso(p.attributes.date)">{{ dateFmt(p.attributes.date) }}</time>
              ·
              <span [style.color]="accent(p.attributes.product)">{{
                productName(p.attributes.product)
              }}</span>
              · {{ typeLabel(p.attributes.type) }}
            </p>
            <h1 class="post__title">{{ p.attributes.title }}</h1>
            <p class="post__summary">{{ p.attributes.summary }}</p>
            <hr class="post__rule" />
            <div class="post__body">
              <analog-markdown [content]="p.content ?? ''" />
            </div>
            @if (p.attributes.prLink || p.attributes.docsLink) {
              <div class="post__links">
                @if (p.attributes.docsLink; as docs) {
                  <a [href]="docs" class="mono post__link" i18n="@@changelog.readDocs"
                    >Read the docs</a
                  >
                }
                @if (p.attributes.prLink; as pr) {
                  <a [href]="pr" class="mono post__link" i18n="@@changelog.seePR">See the PR</a>
                }
              </div>
            }
            <nav class="post__nav">
              <a [href]="backHref" class="mono post__back" i18n="@@changelog.back"
                >Back to changelog</a
              >
            </nav>
          </div>
        </article>
      </flr-site-shell>
    }
  `,
  styles: `
    .post {
      padding: 64px 0 96px;
    }
    .post__container {
      max-width: 720px;
    }
    .post__meta {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--paper-ink-3);
      margin: 0 0 24px;
    }
    .post__title {
      font-family: var(--font-display);
      font-weight: 900;
      font-size: clamp(40px, 6vw, 72px);
      line-height: 1;
      letter-spacing: -0.04em;
      margin: 0 0 16px;
      color: var(--paper-ink);
    }
    .post__summary {
      font-family: var(--font-display);
      font-style: italic;
      font-size: clamp(18px, 1.8vw, 22px);
      line-height: 1.45;
      color: var(--paper-ink-2);
      margin: 0;
      max-width: 600px;
    }
    .post__rule {
      border: none;
      border-top: 1px solid var(--paper-rule);
      margin: 48px 0 40px;
    }
    .post__links {
      display: flex;
      gap: 24px;
      margin: 40px 0 0;
      flex-wrap: wrap;
    }
    .post__link {
      font-size: 12px;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      color: var(--paper-ink-2);
      text-decoration: none;
    }
    .post__link:hover {
      color: var(--paper-ink);
    }
    .post__nav {
      margin-top: 64px;
      padding-top: 24px;
      border-top: 1px solid var(--paper-rule);
    }
    .post__back {
      font-size: 12px;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      color: var(--paper-ink-2);
      text-decoration: none;
    }
    .post__back:hover {
      color: var(--paper-ink);
    }
  `,
})
export class ChangelogPostComponent {
  private readonly locale = resolveLocale(inject(LOCALE_ID));
  private readonly dateLocale = this.locale === 'fr' ? 'fr-FR' : 'en-US';
  private readonly titleSuffix = $localize`:@@changelog.titleSuffix:| FerrFlow Changelog`;

  protected readonly post = toSignal(
    injectContent<ChangelogAttributes>({ param: 'slug', subdirectory: 'changelog' }),
  );

  protected readonly backHref = withLocaleBase(this.locale, '/changelog/');

  protected metaTitle(title: string): string {
    return `${title} ${this.titleSuffix}`;
  }
  protected accent(product: ChangelogAttributes['product']): string {
    return PRODUCT_META[product].accent;
  }
  protected productName(product: ChangelogAttributes['product']): string {
    return PRODUCT_META[product].name;
  }
  protected typeLabel(type: ChangelogAttributes['type']): string {
    return TYPE_LABEL_LONG[type];
  }
  protected iso(date: string): string {
    return new Date(date).toISOString();
  }
  protected dateFmt(date: string): string {
    return new Date(date)
      .toLocaleDateString(this.dateLocale, {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
      })
      .toUpperCase();
  }
}
