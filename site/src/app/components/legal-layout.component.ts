import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  LOCALE_ID,
} from '@angular/core';
import { SiteShellComponent } from '@ferrlabs/ui-ng';

@Component({
  selector: 'flr-legal-layout',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent],
  template: `
    <flr-site-shell [title]="title()" [description]="description()">
      <div class="legal-main">
        @if (draft()) {
          <div class="legal-draft">
            <strong>{{ draftLabel }}</strong> — {{ draftNotice() }}
          </div>
        }
        <header class="legal-header">
          <p class="mono legal-eyebrow">{{ eyebrow() }}</p>
          <h1 class="legal-title">{{ title() }}</h1>
          <p class="mono legal-meta">{{ updatedLabel }}: {{ lastUpdatedFmt() }}</p>
        </header>
        <article class="legal-prose">
          <ng-content />
        </article>
      </div>
    </flr-site-shell>
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
  readonly title = input('');
  readonly description = input('');
  readonly lastUpdated = input('');
  readonly draft = input(false);

  private readonly locale = inject(LOCALE_ID);

  readonly eyebrow = input($localize`:@@legal.eyebrow:Legal document`);
  readonly draftNotice = input(
    $localize`:@@legal.draftNotice:initial text, undergoing legal review. Clauses may change.`,
  );

  protected readonly draftLabel = $localize`:@@legal.draft:Draft`;
  protected readonly updatedLabel = $localize`:@@legal.lastUpdated:Last updated`;

  protected readonly lastUpdatedFmt = computed(() => {
    const raw = this.lastUpdated();
    if (!raw) return '';
    return new Date(raw).toLocaleDateString(this.locale, {
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    });
  });
}
