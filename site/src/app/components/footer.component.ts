import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { Lang, t, TranslationKey } from '../i18n/translations';

interface FooterLink {
  readonly label: string;
  readonly href: string;
  readonly external?: boolean;
}
interface FooterColumn {
  readonly title: string;
  readonly links: readonly FooterLink[];
}

@Component({
  selector: 'flr-site-footer',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <footer class="footer">
      <div class="container">
        <div class="footer__grid">
          <div class="footer__brand">
            <div class="footer__brand-row" translate="no">
              <svg width="40" height="40" viewBox="0 0 32 32" fill="none" aria-hidden="true">
                <line x1="6" y1="16" x2="14" y2="16" stroke="currentColor" stroke-width="1.5" />
                <line x1="18" y1="16" x2="24" y2="10" stroke="currentColor" stroke-width="1.5" />
                <line x1="18" y1="16" x2="24" y2="22" stroke="currentColor" stroke-width="1.5" />
                <circle cx="4" cy="16" r="3" fill="currentColor" />
                <circle cx="16" cy="16" r="3" fill="currentColor" />
                <circle cx="26" cy="10" r="3" fill="currentColor" opacity="0.6" />
                <circle cx="26" cy="22" r="3" fill="currentColor" opacity="0.6" />
              </svg>
              <span class="footer__wordmark">ferrflow</span>
            </div>
            <p class="footer__colophon">{{ tr('footer.tagline') }}</p>
            <a [href]="ferrlabsHref" class="footer__back mono">{{ tr('footer.back') }}</a>
          </div>

          <div class="footer__cols">
            @for (col of columns(); track col.title) {
              <div>
                <div class="mono footer__col-title">{{ col.title }}</div>
                <ul class="footer__col-list">
                  @for (link of col.links; track link.href) {
                    <li>
                      @if (link.external) {
                        <a
                          [href]="link.href"
                          class="footer__link"
                          target="_blank"
                          rel="noreferrer"
                          >{{ link.label }}</a
                        >
                      } @else {
                        <a [href]="link.href" class="footer__link">{{ link.label }}</a>
                      }
                    </li>
                  }
                </ul>
              </div>
            }
          </div>
        </div>

        <div class="footer__bottom">
          <span class="mono footer__bottom-text">{{ tr('footer.rights') }}</span>
          <span class="mono footer__bottom-text">SIREN 104 243 951</span>
        </div>
      </div>
    </footer>
  `,
  styles: `
    .footer {
      padding: 96px 0 48px;
      border-top: 1px solid var(--paper-rule);
      background: var(--paper);
      color: var(--paper-ink);
    }
    .footer__grid {
      display: grid;
      grid-template-columns: minmax(0, 1.4fr) minmax(0, 2fr);
      gap: 64px;
    }
    .footer__brand-row {
      display: flex;
      align-items: center;
      gap: 14px;
      color: var(--accent);
    }
    .footer__wordmark {
      font-family: var(--font-display);
      font-weight: 900;
      font-size: 36px;
      letter-spacing: -0.03em;
      color: var(--paper-ink);
    }
    .footer__colophon {
      font-family: var(--font-display);
      font-style: italic;
      font-size: 18px;
      line-height: 1.5;
      color: var(--paper-ink-2);
      margin: 24px 0 0;
      max-width: 380px;
    }
    .footer__back {
      display: inline-block;
      margin-top: 24px;
      font-size: 12px;
      letter-spacing: 0.06em;
      color: var(--paper-ink-3);
      text-decoration: none;
      transition: color 160ms ease;
    }
    .footer__back:hover {
      color: var(--accent);
    }
    .footer__cols {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 24px;
    }
    .footer__col-title {
      font-size: 11px;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--paper-ink-3);
      margin-bottom: 16px;
    }
    .footer__col-list {
      list-style: none;
      padding: 0;
      margin: 0;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .footer__link {
      font-size: 14px;
      color: var(--paper-ink-2);
      text-decoration: none;
      transition: color 160ms ease;
    }
    .footer__link:hover {
      color: var(--paper-ink);
    }
    .footer__bottom {
      margin-top: 80px;
      padding-top: 24px;
      border-top: 1px solid var(--paper-rule);
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
      gap: 12px;
    }
    .footer__bottom-text {
      font-size: 11px;
      letter-spacing: 0.06em;
      color: var(--paper-ink-3);
    }
    @media (max-width: 880px) {
      .footer__grid {
        grid-template-columns: 1fr;
        gap: 48px;
      }
      .footer__cols {
        grid-template-columns: 1fr 1fr;
      }
    }
  `,
})
export class SiteFooterComponent {
  readonly lang = input<Lang>('en');

  protected readonly ferrlabsHref = 'https://ferrlabs.com';

  protected readonly columns = computed<readonly FooterColumn[]>(() => {
    const lang = this.lang();
    const fr = lang === 'fr';
    return [
      {
        title: t(lang, 'footer.resources'),
        links: [
          {
            label: t(lang, 'footer.documentation'),
            href: fr ? '/fr/docs/introduction' : '/docs/introduction',
          },
          {
            label: t(lang, 'footer.performance'),
            href: fr ? '/fr/performance' : '/performance',
          },
          {
            label: t(lang, 'footer.releases'),
            href: 'https://github.com/FerrLabs/FerrFlow/releases',
            external: true,
          },
          {
            label: t(lang, 'footer.discussions'),
            href: 'https://github.com/FerrLabs/FerrFlow/discussions',
            external: true,
          },
        ],
      },
      {
        title: t(lang, 'footer.product'),
        links: [
          { label: 'GitHub', href: 'https://github.com/FerrLabs/FerrFlow', external: true },
          { label: 'FerrLabs', href: 'https://ferrlabs.com', external: true },
        ],
      },
      {
        title: t(lang, 'footer.legal'),
        links: [
          { label: t(lang, 'footer.mentions'), href: fr ? '/fr/legal/' : '/legal/' },
          { label: t(lang, 'footer.privacy'), href: fr ? '/fr/privacy/' : '/privacy/' },
          { label: t(lang, 'footer.cgu'), href: fr ? '/fr/terms/' : '/terms/' },
          { label: t(lang, 'footer.cookies'), href: fr ? '/fr/cookies/' : '/cookies/' },
          { label: t(lang, 'footer.security'), href: fr ? '/fr/security/' : '/security/' },
        ],
      },
    ];
  });

  protected tr(key: TranslationKey): string {
    return t(this.lang(), key);
  }
}
