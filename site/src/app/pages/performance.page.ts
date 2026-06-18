import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ShellComponent } from '../components/shell.component';
import { PerformanceComponent } from '../components/performance.component';

@Component({
  selector: 'flr-performance-en',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ShellComponent, PerformanceComponent],
  template: `
    <flr-shell
      lang="en"
      title="Performance — FerrFlow"
      description="How FerrFlow compares to semantic-release, changesets, standard-version and commit-and-tag-version on real fixtures. Median latency, peak memory, install size — measured with hyperfine in CI."
      canonical="https://ferrflow.com/performance/"
      enHref="/performance/"
      frHref="/fr/performance/"
    >
      <flr-performance lang="en" />
    </flr-shell>
  `,
})
export default class PerformanceEnPage {}
