import { ChangeDetectionStrategy, Component } from '@angular/core';
import { LegalLayoutComponent } from '../../components/legal-layout.component';

@Component({
  selector: 'flr-cookies-fr',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LegalLayoutComponent],
  template: `
    <flr-legal-layout
      lang="fr"
      title="Cookies — FerrFlow"
      description="ferrflow.com ne dépose aucun cookie de son propre fait. Point."
      canonical="https://ferrflow.com/fr/cookies/"
      lastUpdated="2026-05-01"
      enHref="/cookies/"
      frHref="/fr/cookies/"
    >
      <p>
        ferrflow.com ne dépose <strong>aucun cookie de son propre fait</strong>. Le seul mécanisme
        utilisé est le <code>localStorage</code> du navigateur pour mémoriser votre choix de langue.
      </p>
      <p>Pas de cookie d'analytics. Pas de traceur marketing. Pas de script tiers.</p>
      <p>
        Le CLI lui-même est un logiciel offline — pas de cookies, pas d'appels réseau au-delà de ceux
        que vous invoquez explicitement.
      </p>
      <p>
        Si vous vous connectez à votre compte FerrLabs sur <code>app.ferrlabs.com</code>, la
        <a href="https://ferrlabs.com/fr/cookies">politique cookies FerrLabs canonique</a> s'applique.
      </p>

      <p style="margin-top:48px;font-size:13px;">
        English version: <a href="/cookies">Cookies</a>.
      </p>
    </flr-legal-layout>
  `,
})
export default class CookiesFrPage {}
