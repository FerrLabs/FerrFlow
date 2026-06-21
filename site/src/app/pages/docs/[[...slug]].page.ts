import { ChangeDetectionStrategy, Component } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { injectContent, MarkdownComponent } from '@analogjs/content';
import { DocsPageComponent } from '../../docs/docs-page.component';

interface DocAttributes {
  readonly title: string;
  readonly description: string;
}

@Component({
  selector: 'flr-docs-page-en',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [DocsPageComponent, MarkdownComponent],
  template: `
    @if (post(); as post) {
      <flr-docs-page
        lang="en"
        [slug]="post.slug"
        [title]="post.attributes.title"
        [description]="post.attributes.description"
      >
        <analog-markdown [content]="post.content ?? ''" />
      </flr-docs-page>
    }
  `,
})
export default class DocsPageEn {
  protected readonly post = toSignal(
    injectContent<DocAttributes>({ param: 'slug', subdirectory: 'docs-en' }),
  );
}
