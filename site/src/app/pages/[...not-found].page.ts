import { ChangeDetectionStrategy, Component } from '@angular/core';
import { SiteShellComponent } from '@ferrlabs/ui-ng';

@Component({
  selector: 'flr-not-found',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent],
  template: `
    <flr-site-shell [title]="title" [description]="description">
      <section class="error-page">
        <p class="kicker" i18n="@@notfound.kicker">Error 404</p>
        <h1 i18n="@@notfound.heading">Page not found</h1>
        <p class="lede" i18n="@@notfound.lede">
          The route you tried doesn't exist on ferrflow.com.
        </p>
        <ul class="links">
          <li><a href="/" i18n="@@notfound.back">← Back to ferrflow.com</a></li>
          <li><a href="/docs/introduction/" i18n="@@notfound.docs">Read the docs</a></li>
          <li>
            <a href="https://github.com/FerrLabs/FerrFlow/releases" i18n="@@notfound.releases"
              >Latest releases</a
            >
          </li>
        </ul>
      </section>
    </flr-site-shell>
  `,
  styles: `
    .error-page {
      max-width: 560px;
      margin: 0 auto;
      padding: 96px 24px;
      text-align: center;
      color: var(--color-ink-2, #475569);
    }
    .kicker {
      font-family: var(--font-mono, ui-monospace, monospace);
      font-size: 11px;
      letter-spacing: 0.32em;
      text-transform: uppercase;
      color: var(--color-ink-3, #94a3b8);
      margin: 0 0 18px;
    }
    h1 {
      font-size: clamp(32px, 5vw, 48px);
      font-weight: 700;
      letter-spacing: -0.025em;
      color: var(--color-ink, #0f172a);
      margin: 0 0 16px;
      line-height: 1.05;
    }
    .lede {
      font-size: 16px;
      margin: 0 0 32px;
    }
    .links {
      list-style: none;
      padding: 0;
      margin: 0;
      display: flex;
      flex-direction: column;
      gap: 10px;
    }
    .links a {
      color: #e8733a;
      text-decoration: underline;
      text-underline-offset: 3px;
    }
  `,
})
export default class NotFoundPage {
  protected readonly title = $localize`:@@notfound.metaTitle:404: FerrFlow`;
  protected readonly description = $localize`:@@notfound.metaDescription:This page doesn't exist.`;
}
