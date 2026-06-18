import { ChangeDetectionStrategy, Component } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { injectContent, MarkdownComponent } from '@analogjs/content';
import { DocsLayoutComponent } from '../../../../docs/docs-layout.component';

interface DocAttributes {
  readonly title: string;
  readonly description: string;
}

@Component({
  selector: 'flr-docs-page-fr-v4',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [DocsLayoutComponent, MarkdownComponent],
  template: `
    @if (post(); as post) {
      <flr-docs-layout
        lang="fr"
        version="v4"
        [slug]="post.slug"
        [title]="post.attributes.title"
        [description]="post.attributes.description"
      >
        <analog-markdown [content]="post.content ?? ''" />
      </flr-docs-layout>
    }
  `,
})
export default class DocsPageFrV4 {
  protected readonly post = toSignal(
    injectContent<DocAttributes>({ param: 'slug', subdirectory: 'docs-fr-v4' }),
  );
}
