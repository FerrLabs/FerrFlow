import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ChangelogPostComponent } from '../../changelog/changelog-post.component';

@Component({
  selector: 'flr-changelog-post-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ChangelogPostComponent],
  template: `<flr-changelog-post />`,
})
export default class ChangelogPostPage {}
