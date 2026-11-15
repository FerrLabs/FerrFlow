import { ChangeDetectionStrategy, Component, computed, inject, LOCALE_ID } from '@angular/core';
import { injectContentFiles } from '@analogjs/content';
import { SiteShellComponent, resolveLocale, withLocaleBase } from '@ferrlabs/ui-ng';
import { ChangelogAttributes, CHANGELOG_PRODUCT, TYPE_LABEL_SHORT } from './changelog.types';

interface Entry {
  readonly slug: string;
  readonly href: string;
  readonly date: Date;
  readonly attributes: ChangelogAttributes;
}
interface MonthGroup {
  readonly key: string;
  readonly label: string;
  readonly entries: readonly Entry[];
}

@Component({
  selector: 'flr-changelog-index',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent],
  template: `
    <flr-site-shell [title]="metaTitle" [description]="metaDesc">
      <section class="cl-hero">
        <div class="container">
          <p class="mono cl-eyebrow" i18n="@@changelog.eyebrow">Changelog</p>
          <h1 class="cl-title">
            <span i18n="@@changelog.titleA">What we</span><br />
            <span class="italic" i18n="@@changelog.titleB">shipped.</span>
          </h1>
          <div class="cl-intro">
            <p class="cl-sub" i18n="@@changelog.intro">
              Every feature, fix and improvement to FerrFlow, the versioning CLI and the site around
              it.
            </p>
            <a [href]="rssHref" class="mono cl-rss" i18n="@@changelog.rss">RSS</a>
          </div>
        </div>
      </section>

      <section class="cl-list">
        <div class="container">
          @for (group of groups(); track group.key) {
            <div class="cl-month">
              <div class="cl-month-divider">
                <span class="mono cl-month-label">── {{ group.label }} ──</span>
              </div>
              @for (entry of group.entries; track entry.slug) {
                <a [href]="entry.href" class="cl-entry">
                  <div class="cl-entry-meta">
                    <time class="mono cl-entry-date" [attr.datetime]="entry.date.toISOString()">
                      {{ dateShort(entry.date) }}
                    </time>
                    <span class="mono cl-entry-type">
                      {{ typeLabel(entry.attributes.type) }}
                    </span>
                  </div>
                  <div class="cl-entry-body">
                    <h2 class="cl-entry-title">{{ entry.attributes.title }}</h2>
                    <p class="cl-entry-summary">{{ entry.attributes.summary }}</p>
                  </div>
                  <span class="cl-entry-arrow" aria-hidden="true">→</span>
                </a>
              }
            </div>
          }
        </div>
      </section>
    </flr-site-shell>
  `,
  styles: `
    .cl-hero {
      padding: 96px 0 48px;
    }
    .cl-eyebrow {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--paper-ink-3);
      margin: 0 0 24px;
    }
    .cl-title {
      font-family: var(--font-display);
      font-weight: 900;
      font-size: clamp(56px, 11vw, 168px);
      line-height: 0.92;
      letter-spacing: -0.045em;
      margin: 0;
    }
    .cl-intro {
      display: grid;
      grid-template-columns: minmax(0, 1.2fr) auto;
      gap: 64px;
      align-items: end;
      margin-top: 48px;
    }
    .cl-sub {
      font-size: clamp(17px, 1.5vw, 21px);
      line-height: 1.55;
      color: var(--paper-ink-2);
      max-width: 600px;
      margin: 0;
    }
    .cl-rss {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--paper-ink-2);
      text-decoration: none;
      border-bottom: 1px solid var(--paper-rule-strong);
      padding-bottom: 4px;
      white-space: nowrap;
    }
    @media (max-width: 760px) {
      .cl-intro {
        grid-template-columns: 1fr;
        gap: 24px;
      }
    }
    .cl-list {
      padding: 32px 0 96px;
      border-top: 1px solid var(--paper-rule);
    }
    .cl-month {
      margin-top: 48px;
    }
    .cl-month-divider {
      padding: 0 0 16px;
    }
    .cl-month-label {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--paper-ink-3);
    }
    .cl-entry {
      display: grid;
      grid-template-columns: 200px minmax(0, 1fr) auto;
      gap: 32px;
      align-items: start;
      padding: 28px 0;
      border-bottom: 1px solid var(--paper-rule);
      color: var(--paper-ink);
      text-decoration: none;
      transition: background 200ms;
    }
    .cl-entry:hover {
      background: color-mix(in oklab, var(--paper-ink) 3%, transparent);
    }
    .cl-entry:hover .cl-entry-arrow {
      transform: translateX(4px);
    }
    .cl-entry-meta {
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
    .cl-entry-date {
      font-size: 11px;
      letter-spacing: 0.14em;
      color: var(--paper-ink-3);
    }
    .cl-entry-type {
      font-size: 10.5px;
      letter-spacing: 0.16em;
      font-weight: 500;
      color: var(--accent);
    }
    .cl-entry-title {
      font-family: var(--font-display);
      font-weight: 700;
      font-size: clamp(22px, 2.5vw, 30px);
      letter-spacing: -0.02em;
      line-height: 1.15;
      margin: 0 0 8px;
    }
    .cl-entry-summary {
      font-size: 15.5px;
      line-height: 1.55;
      color: var(--paper-ink-2);
      margin: 0;
    }
    .cl-entry-arrow {
      font-family: var(--font-mono);
      font-size: 16px;
      color: var(--paper-ink-3);
      transition: transform 200ms;
      align-self: center;
    }
    @media (max-width: 760px) {
      .cl-entry {
        grid-template-columns: 1fr;
        gap: 12px;
      }
      .cl-entry-arrow {
        display: none;
      }
      .cl-entry-meta {
        flex-direction: row;
      }
    }
  `,
})
export class ChangelogIndexComponent {
  private readonly locale = resolveLocale(inject(LOCALE_ID));
  private readonly dateLocale = this.locale === 'fr' ? 'fr-FR' : 'en-US';

  protected readonly metaTitle = $localize`:@@changelog.metaTitle:Changelog, FerrFlow`;
  protected readonly metaDesc = $localize`:@@changelog.metaDesc:Every feature, fix and improvement to FerrFlow, the versioning CLI and the site around it.`;
  protected readonly rssHref = withLocaleBase(this.locale, '/changelog/rss.xml');

  private readonly files = injectContentFiles<ChangelogAttributes>(
    (file) => !file.attributes.draft && file.attributes.product === CHANGELOG_PRODUCT,
  );

  private readonly base = withLocaleBase(this.locale, '/changelog');

  private readonly entries = computed<Entry[]>(() => {
    const base = this.base;
    return this.files
      .map((file) => ({
        slug: file.slug,
        href: `${base}/${file.slug}/`,
        date: new Date(file.attributes.date),
        attributes: file.attributes,
      }))
      .sort((a, b) => b.date.getTime() - a.date.getTime());
  });

  protected readonly groups = computed<MonthGroup[]>(() => {
    const locale = this.dateLocale;
    const groups: MonthGroup[] = [];
    for (const entry of this.entries()) {
      const key = `${entry.date.getFullYear()}-${String(entry.date.getMonth() + 1).padStart(2, '0')}`;
      let group = groups.find((g) => g.key === key) as
        { key: string; label: string; entries: Entry[] } | undefined;
      if (!group) {
        group = {
          key,
          label: entry.date
            .toLocaleDateString(locale, { month: 'long', year: 'numeric' })
            .toUpperCase(),
          entries: [],
        };
        groups.push(group);
      }
      group.entries.push(entry);
    }
    return groups;
  });

  protected typeLabel(type: ChangelogAttributes['type']): string {
    return TYPE_LABEL_SHORT[type];
  }
  protected dateShort(date: Date): string {
    return date
      .toLocaleDateString(this.dateLocale, { month: 'short', day: 'numeric' })
      .toUpperCase();
  }
}
