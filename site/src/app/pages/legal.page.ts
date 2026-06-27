import { ChangeDetectionStrategy, Component } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { injectContent, MarkdownComponent } from '@analogjs/content';
import { injectLocale } from '@analogjs/router/tokens';
import { LegalLayoutComponent } from '../components/legal-layout.component';

interface LegalAttributes {
  readonly title: string;
  readonly description: string;
  readonly lastUpdated: string;
}

@Component({
  selector: 'flr-legal',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LegalLayoutComponent, MarkdownComponent],
  template: `
    @if (post(); as post) {
      <flr-legal-layout
        [title]="post.attributes.title"
        [description]="post.attributes.description"
        [lastUpdated]="post.attributes.lastUpdated"
      >
        <analog-markdown [content]="post.content ?? ''" />
      </flr-legal-layout>
    }
  `,
})
export default class LegalPage {
  private readonly dir = injectLocale() === 'fr' ? 'legal-fr' : 'legal-en';

  protected readonly post = toSignal(
    injectContent<LegalAttributes>({ customFilename: `${this.dir}/legal` }),
  );
}
