import { ChangeDetectionStrategy, Component, input, signal } from '@angular/core';
import { Lang, t, TranslationKey } from '../i18n/translations';

@Component({
  selector: 'flr-site-navbar',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '(window:scroll)': 'onScroll()',
  },
  template: `
    <header class="nav" [class.nav--scrolled]="scrolled()" data-theme-aware>
      <div class="nav__container">
        <a [href]="homeHref()" class="nav__brand" translate="no">
          <svg width="28" height="28" viewBox="0 0 32 32" fill="none" aria-hidden="true">
            <line x1="6" y1="16" x2="14" y2="16" stroke="currentColor" stroke-width="1.5" />
            <line x1="18" y1="16" x2="24" y2="10" stroke="currentColor" stroke-width="1.5" />
            <line x1="18" y1="16" x2="24" y2="22" stroke="currentColor" stroke-width="1.5" />
            <circle cx="4" cy="16" r="3" fill="currentColor" />
            <circle cx="16" cy="16" r="3" fill="currentColor" />
            <circle cx="26" cy="10" r="3" fill="currentColor" opacity="0.6" />
            <circle cx="26" cy="22" r="3" fill="currentColor" opacity="0.6" />
          </svg>
          <span class="nav__wordmark">ferrflow</span>
        </a>

        <nav class="nav__desktop">
          <a [href]="docsHref()" class="nav__link mono">{{ tr('nav.docs') }}</a>
          <a [href]="performanceHref()" class="nav__link mono">{{ tr('nav.performance') }}</a>
          <a
            href="https://github.com/FerrLabs/FerrFlow"
            target="_blank"
            rel="noreferrer"
            class="nav__link mono"
            >{{ tr('nav.github') }}</a
          >

          <div class="nav__lang mono" translate="no">
            @if (enAvailable()) {
              <a
                [href]="enHref()"
                class="nav__lang-opt"
                [class.is-active]="lang() === 'en'"
                (click)="rememberLang('en')"
                >EN</a
              >
            } @else {
              <span
                class="nav__lang-opt is-disabled"
                aria-disabled="true"
                [title]="tr('lang.unavailable')"
                >EN</span
              >
            }
            @if (frAvailable()) {
              <a
                [href]="frHref()"
                class="nav__lang-opt"
                [class.is-active]="lang() === 'fr'"
                (click)="rememberLang('fr')"
                >FR</a
              >
            } @else {
              <span
                class="nav__lang-opt is-disabled"
                aria-disabled="true"
                [title]="tr('lang.unavailable')"
                >FR</span
              >
            }
          </div>

          <a [href]="docsHref()" class="nav__cta mono">
            {{ tr('nav.cta') }}<span aria-hidden="true">→</span>
          </a>
        </nav>

        <button
          class="nav__toggle mono"
          type="button"
          aria-label="Open menu"
          (click)="mobileOpen.set(true)"
        >
          Menu
        </button>
      </div>

      @if (mobileOpen()) {
        <div class="nav__mobile">
          <div class="nav__container nav__mobile-bar">
            <a [href]="homeHref()" class="nav__brand" translate="no">
              <svg width="28" height="28" viewBox="0 0 32 32" fill="none" aria-hidden="true">
                <line x1="6" y1="16" x2="14" y2="16" stroke="currentColor" stroke-width="1.5" />
                <line x1="18" y1="16" x2="24" y2="10" stroke="currentColor" stroke-width="1.5" />
                <line x1="18" y1="16" x2="24" y2="22" stroke="currentColor" stroke-width="1.5" />
                <circle cx="4" cy="16" r="3" fill="currentColor" />
                <circle cx="16" cy="16" r="3" fill="currentColor" />
                <circle cx="26" cy="10" r="3" fill="currentColor" opacity="0.6" />
                <circle cx="26" cy="22" r="3" fill="currentColor" opacity="0.6" />
              </svg>
              <span class="nav__wordmark">ferrflow</span>
            </a>
            <button class="nav__toggle mono" type="button" (click)="mobileOpen.set(false)">
              Close
            </button>
          </div>
          <div class="nav__container nav__mobile-list">
            <a [href]="docsHref()" class="nav__mobile-link" (click)="mobileOpen.set(false)">
              <span class="mono">01</span>{{ tr('nav.docs') }}
            </a>
            <a [href]="performanceHref()" class="nav__mobile-link" (click)="mobileOpen.set(false)">
              <span class="mono">02</span>{{ tr('nav.performance') }}
            </a>
            <a
              href="https://github.com/FerrLabs/FerrFlow"
              class="nav__mobile-link"
              (click)="mobileOpen.set(false)"
            >
              <span class="mono">03</span>{{ tr('nav.github') }}
            </a>
          </div>
        </div>
      }
    </header>
  `,
  styles: `
    .nav {
      position: sticky;
      top: 0;
      z-index: 50;
      background: rgba(250, 248, 244, 0.65);
      -webkit-backdrop-filter: saturate(140%) blur(14px);
      backdrop-filter: saturate(140%) blur(14px);
      border-bottom: 1px solid transparent;
      transition:
        background 200ms ease,
        border-color 200ms ease;
    }
    .nav--scrolled {
      background: rgba(250, 248, 244, 0.85);
      border-bottom-color: var(--color-rule);
    }
    .nav__container {
      max-width: 1440px;
      margin: 0 auto;
      padding: 0 40px;
      display: flex;
      align-items: center;
      height: 72px;
      gap: 24px;
    }
    @media (max-width: 768px) {
      .nav__container {
        padding: 0 20px;
      }
    }
    .nav__brand {
      display: flex;
      align-items: center;
      gap: 10px;
      color: var(--accent);
      letter-spacing: -0.01em;
      text-decoration: none;
    }
    .nav__wordmark {
      font-family: var(--font-display);
      font-weight: 900;
      font-size: 22px;
      letter-spacing: -0.02em;
      color: var(--color-ink);
    }
    .nav__desktop {
      display: flex;
      align-items: center;
      gap: 28px;
      margin-left: auto;
    }
    .nav__link {
      font-size: 12px;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      color: var(--color-ink-2);
      position: relative;
      padding: 6px 0;
      text-decoration: none;
    }
    .nav__link::after {
      content: '';
      position: absolute;
      left: 0;
      right: 0;
      bottom: -2px;
      height: 1px;
      background: var(--accent);
      transform: scaleX(0);
      transform-origin: left;
      transition: transform 220ms ease;
    }
    .nav__link:hover::after {
      transform: scaleX(1);
    }
    .nav__lang {
      display: flex;
      align-items: center;
      border: 1px solid var(--color-rule);
      border-radius: 999px;
      overflow: hidden;
    }
    .nav__lang-opt {
      font-family: var(--font-mono);
      font-size: 11px;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      padding: 6px 12px;
      color: var(--color-ink-2);
      text-decoration: none;
      transition:
        background 180ms,
        color 180ms;
    }
    .nav__lang-opt.is-active {
      background: var(--accent);
      color: #fff;
    }
    .nav__lang-opt.is-disabled {
      opacity: 0.4;
      text-decoration: line-through;
      cursor: not-allowed;
    }
    .nav__cta {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      font-size: 12px;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      padding: 10px 16px;
      border-radius: 999px;
      background: var(--accent);
      color: #fff;
      text-decoration: none;
      transition:
        transform 180ms,
        opacity 180ms;
    }
    .nav__cta:hover {
      transform: translateY(-1px);
      opacity: 0.92;
    }
    .nav__toggle {
      display: none;
      margin-left: auto;
      border: 1px solid var(--color-rule);
      border-radius: 999px;
      background: transparent;
      color: var(--color-ink);
      padding: 8px 14px;
      font-size: 11px;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      cursor: pointer;
    }
    @media (max-width: 880px) {
      .nav__desktop {
        display: none;
      }
      .nav__toggle {
        display: inline-flex;
      }
    }
    .nav__mobile {
      position: fixed;
      inset: 0;
      z-index: 100;
      background: var(--color-paper);
    }
    .nav__mobile-bar {
      justify-content: space-between;
    }
    .nav__mobile-list {
      flex-direction: column;
      align-items: stretch;
      height: auto;
      padding-top: 40px;
      padding-bottom: 40px;
      gap: 0;
    }
    .nav__mobile-link {
      font-family: var(--font-display);
      font-weight: 700;
      font-size: clamp(40px, 9vw, 72px);
      letter-spacing: -0.03em;
      line-height: 1.04;
      border-bottom: 1px solid var(--color-rule);
      padding: 20px 0;
      color: var(--color-ink);
      display: flex;
      align-items: baseline;
      gap: 16px;
      text-decoration: none;
    }
    .nav__mobile-link .mono {
      font-size: 12px;
      color: var(--color-ink-3);
      letter-spacing: 0.1em;
      font-weight: 400;
    }
  `,
})
export class SiteNavbarComponent {
  readonly lang = input<Lang>('en');
  readonly enHref = input('/');
  readonly frHref = input('/fr/');
  readonly enAvailable = input(true);
  readonly frAvailable = input(true);

  protected readonly scrolled = signal(false);
  protected readonly mobileOpen = signal(false);

  protected homeHref(): string {
    return this.lang() === 'fr' ? '/fr/' : '/';
  }
  protected docsHref(): string {
    return this.lang() === 'fr' ? '/fr/docs/introduction' : '/docs/introduction';
  }
  protected performanceHref(): string {
    return this.lang() === 'fr' ? '/fr/performance' : '/performance';
  }

  protected tr(key: TranslationKey): string {
    return t(this.lang(), key);
  }

  protected onScroll(): void {
    this.scrolled.set(typeof window !== 'undefined' && window.scrollY > 8);
  }

  protected rememberLang(lang: Lang): void {
    try {
      localStorage.setItem('lang-pref', lang);
    } catch {
      /* no-op */
    }
  }
}
