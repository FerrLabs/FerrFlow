import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ChangelogIndexComponent } from '../../changelog/changelog-index.component';

@Component({
  selector: 'flr-changelog-index-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ChangelogIndexComponent],
  template: `<flr-changelog-index />`,
})
export default class ChangelogIndexPage {}
