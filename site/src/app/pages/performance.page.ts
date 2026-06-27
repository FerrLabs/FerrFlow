import { ChangeDetectionStrategy, Component } from '@angular/core';
import { SiteShellComponent } from '@ferrlabs/ui-ng';
import { PerformanceComponent } from '../components/performance.component';

@Component({
  selector: 'flr-performance-en',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SiteShellComponent, PerformanceComponent],
  template: `
    <flr-site-shell [title]="title" [description]="description">
      <flr-performance />
    </flr-site-shell>
  `,
})
export default class PerformancePage {
  protected readonly title = $localize`:@@perf.metaTitle:Performance — FerrFlow`;
  protected readonly description = $localize`:@@perf.metaDescription:How FerrFlow compares to semantic-release, changesets, standard-version and commit-and-tag-version on real fixtures. Median latency, peak memory, install size — measured with hyperfine in CI.`;
}
