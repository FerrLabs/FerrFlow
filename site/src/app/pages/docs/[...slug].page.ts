import { ChangeDetectionStrategy, Component } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { injectContent, MarkdownComponent } from '@analogjs/content';
import { DocsLayoutComponent } from '../../docs/docs-layout.component';

interface DocAttributes {
  readonly title: string;
  readonly description: string;
}

@Component({
  selector: 'flr-docs-page-en',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [DocsLayoutComponent, MarkdownComponent],
  template: `
    @if (post(); as post) {
      <flr-docs-layout
        lang="en"
        [slug]="post.slug"
        [title]="post.attributes.title"
        [description]="post.attributes.description"
      >
        <analog-markdown [content]="post.content ?? ''" />
      </flr-docs-layout>
    }
  `,
})
export default class DocsPageEn {
  protected readonly post = toSignal(
    injectContent<DocAttributes>({ param: 'slug', subdirectory: 'docs' }),
  );
}
