import { ChangeDetectionStrategy, Component } from '@angular/core';
import { LegalLayoutComponent } from '../../components/legal-layout.component';

@Component({
  selector: 'flr-legal-fr',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LegalLayoutComponent],
  template: `
    <flr-legal-layout
      lang="fr"
      title="Mentions légales — FerrFlow"
      description="Identité de l'éditeur, hébergement et directeur de publication de ferrflow.com."
      canonical="https://ferrflow.com/fr/legal/"
      lastUpdated="2026-05-06"
      enHref="/legal/"
      frHref="/fr/legal/"
    >
      <h2>Éditeur</h2>
      <p>
        Bryan Ferrando — Entrepreneur individuel<br />
        Nom commercial : <strong>FerrLabs</strong>.
      </p>
      <p>
        187 avenue Charles Saint-Venant, Appartement 64<br />
        59800 Lille, France
      </p>
      <ul>
        <li>SIREN : 104 243 951</li>
        <li>SIRET (établissement principal) : 10424395100019</li>
        <li>Code APE : 6201Z (Programmation informatique)</li>
        <li>TVA non applicable, article 293 B du CGI</li>
      </ul>
      <p>
        Email : contact&#64;ferrlabs.com<br />
        Téléphone : +33 7 69 31 58 91
      </p>

      <h2>Directeur de publication</h2>
      <p>Bryan Ferrando.</p>

      <h2>Hébergeur</h2>
      <p>
        OVH SAS<br />
        2 rue Kellermann, 59100 Roubaix, France<br />
        Téléphone : +33 9 72 10 10 07<br />
        Site : ovh.com
      </p>

      <h2>Propriété intellectuelle</h2>
      <p>
        L'ensemble des contenus présents sur ce site (textes, graphismes, logos, marques) sont la
        propriété exclusive de FerrLabs ou de leurs ayants droit, sauf mention contraire explicite.
        Toute reproduction, distribution, modification ou utilisation, même partielle, sans
        autorisation écrite préalable est interdite et constitue une contrefaçon sanctionnée par les
        articles L.335-2 et suivants du Code de la propriété intellectuelle.
      </p>
      <p>
        Le CLI FerrFlow lui-même est un logiciel <strong>open source</strong> distribué sous licence
        <strong>Mozilla Public License 2.0</strong> (MPL-2.0). Le code source et le texte intégral de
        la licence sont disponibles sur
        <a href="https://github.com/FerrLabs/FerrFlow">github.com/FerrLabs/FerrFlow</a>.
      </p>

      <h2>Liens externes</h2>
      <p>
        Ce site peut contenir des liens vers des sites tiers. FerrLabs n'exerce aucun contrôle sur
        ces sites et décline toute responsabilité quant à leur contenu.
      </p>

      <h2>Droit applicable</h2>
      <p>
        Les présentes mentions sont régies par le droit français. Tout litige relève de la
        compétence exclusive des tribunaux du ressort de Lille.
      </p>

      <p style="margin-top:48px;font-size:13px;">
        English version: <a href="/legal">Legal notice</a>.
      </p>
    </flr-legal-layout>
  `,
})
export default class LegalFrPage {}
