import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { Lang } from '../i18n/translations';
import { ShellComponent } from './shell.component';

@Component({
  selector: 'flr-legal-layout',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ShellComponent],
  template: `
    <flr-shell
      [lang]="lang()"
      [title]="title()"
      [description]="description()"
      [canonical]="canonical()"
      [enHref]="enHref()"
      [frHref]="frHref()"
    >
      <div class="legal-main">
        @if (draft()) {
          <div class="legal-draft">
            <strong>{{ lang() === 'fr' ? 'Brouillon' : 'Draft' }}</strong> — {{ draftNotice() }}
          </div>
        }
        <header class="legal-header">
          <p class="mono legal-eyebrow">{{ eyebrow() }}</p>
          <h1 class="legal-title">{{ title() }}</h1>
          <p class="mono legal-meta">{{ updatedLabel() }}: {{ lastUpdatedFmt() }}</p>
        </header>
        <article class="legal-prose">
          <ng-content />
        </article>
      </div>
    </flr-shell>
  `,
  styles: `
    .legal-main {
      max-width: 760px;
      margin: 0 auto;
      padding: 64px 24px 96px;
    }
    .legal-draft {
      margin-bottom: 32px;
      padding: 14px 18px;
      background: rgba(232, 115, 58, 0.08);
      border: 1px solid rgba(232, 115, 58, 0.4);
      border-radius: 10px;
      font-family: var(--font-mono);
      font-size: 12px;
      line-height: 1.55;
      color: var(--paper-ink-2);
    }
    .legal-draft strong {
      color: #c85a2c;
      font-family: var(--font-display);
      font-style: italic;
      font-size: 14px;
      text-transform: none;
      letter-spacing: 0;
    }
    .legal-header {
      margin: 24px 0 56px;
    }
    .legal-eyebrow {
      font-size: 11px;
      letter-spacing: 0.32em;
      text-transform: uppercase;
      color: var(--paper-ink-3);
      margin: 0 0 14px;
    }
    .legal-title {
      font-family: var(--font-display);
      font-weight: 700;
      line-height: 0.96;
      letter-spacing: -0.025em;
      color: var(--paper-ink);
      font-size: clamp(40px, 6vw, 60px);
      margin: 0;
    }
    .legal-meta {
      font-size: 11px;
      letter-spacing: 0.18em;
      color: var(--paper-ink-3);
      margin-top: 14px;
    }
  `,
})
export class LegalLayoutComponent {
  readonly lang = input<Lang>('en');
  readonly title = input('');
  readonly description = input('');
  readonly canonical = input<string | null>(null);
  readonly lastUpdated = input('');
  readonly draft = input(false);
  readonly enHref = input('/');
  readonly frHref = input('/fr/');

  protected readonly eyebrow = computed(() =>
    this.lang() === 'fr' ? 'Document légal' : 'Legal document',
  );
  protected readonly updatedLabel = computed(() =>
    this.lang() === 'fr' ? 'Dernière mise à jour' : 'Last updated',
  );
  protected readonly draftNotice = computed(() =>
    this.lang() === 'fr'
      ? 'texte initial, en cours de validation juridique. Les clauses peuvent évoluer.'
      : 'initial text, undergoing legal review. Clauses may change.',
  );
  protected readonly lastUpdatedFmt = computed(() => {
    const raw = this.lastUpdated();
    if (!raw) return '';
    const locale = this.lang() === 'fr' ? 'fr-FR' : 'en-US';
    return new Date(raw).toLocaleDateString(locale, {
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    });
  });
}
