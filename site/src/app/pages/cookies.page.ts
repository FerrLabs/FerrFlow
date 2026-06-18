import { ChangeDetectionStrategy, Component } from '@angular/core';
import { LegalLayoutComponent } from '../components/legal-layout.component';

@Component({
  selector: 'flr-cookies-en',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LegalLayoutComponent],
  template: `
    <flr-legal-layout
      lang="en"
      title="Cookies — FerrFlow"
      description="ferrflow.com sets no cookie of its own. Period."
      canonical="https://ferrflow.com/cookies/"
      lastUpdated="2026-05-01"
      enHref="/cookies/"
      frHref="/fr/cookies/"
    >
      <p>
        ferrflow.com sets <strong>no cookie of its own</strong>. The only mechanism in use is the
        browser <code>localStorage</code> for remembering your language preference.
      </p>
      <p>No analytics cookie. No marketing tracker. No third-party script.</p>
      <p>
        The CLI itself is offline software — no cookies, no network calls beyond what you explicitly
        invoke.
      </p>
      <p>
        If you sign in to your FerrLabs account at <code>app.ferrlabs.com</code>, the
        <a href="https://ferrlabs.com/cookies">canonical FerrLabs cookie policy</a> applies there.
      </p>

      <p style="margin-top:48px;font-size:13px;">
        French version: <a href="/fr/cookies">Cookies</a>.
      </p>
    </flr-legal-layout>
  `,
})
export default class CookiesEnPage {}
